"""Bounded decoder for Perfetto v15 TracingServiceState query replies.

This does not admit devices, start tracing, or prove historical inactivity.
ProtoZero's four-byte nested-message length padding is supported explicitly.
"""
import argparse
import json
from pathlib import Path

MAX_QUERY_BYTES = 262144
MAX_FIELDS = 4096
REQUIRED = frozenset({
    "android.surfaceflinger.frametimeline", "linux.ftrace", "linux.process_stats",
})


def varint(raw, index, *, protozero_length=False):
    start, value = index, 0
    for shift in range(0, 70, 7):
        if index >= len(raw):
            raise ValueError("Truncated protobuf varint")
        byte = raw[index]
        index += 1
        if shift == 63 and byte > 1:
            raise ValueError("Overflowing protobuf varint")
        value |= (byte & 127) << shift
        if not byte & 128:
            if index - start > 1 and byte == 0:
                if not (protozero_length and index - start == 4):
                    raise ValueError("Unsupported redundant protobuf varint")
            return value, index
    raise ValueError("Unterminated protobuf varint")


def fields(raw):
    if not isinstance(raw, bytes) or len(raw) > MAX_QUERY_BYTES:
        raise ValueError("Protobuf exceeds bound")
    rows, index, count = {}, 0, 0
    while index < len(raw):
        tag, index = varint(raw, index)
        number, wire = tag >> 3, tag & 7
        if not 1 <= number < 2**29 or wire not in (0, 1, 2, 5):
            raise ValueError("Unsupported protobuf field")
        if wire == 0:
            value, index = varint(raw, index)
        else:
            if wire == 2:
                size, index = varint(raw, index, protozero_length=True)
            else:
                size = 8 if wire == 1 else 4
            if size > len(raw) - index:
                raise ValueError("Truncated protobuf payload")
            value = raw[index:index + size]
            index += size
        count += 1
        if count > MAX_FIELDS:
            raise ValueError("Too many protobuf fields")
        rows.setdefault(number, []).append((wire, value))
    return rows


def one(rows, key, wire):
    values = rows.get(key, [])
    if len(values) != 1 or values[0][0] != wire:
        raise ValueError("Missing or ambiguous protobuf field")
    return values[0][1]


def service_state(raw):
    top = fields(raw)
    total, started = one(top, 3, 0), one(top, 4, 0)
    if total > 1000 or started > total:
        raise ValueError("Invalid current session counts")
    producers = {}
    for wire, value in top.get(1, []):
        if wire != 2:
            raise ValueError("Producer protobuf wire mismatch")
        producer = fields(value)
        identifier, uid = one(producer, 1, 0), one(producer, 3, 0)
        name = one(producer, 2, 2).decode("utf-8")
        if (not 0 < identifier < 2**31 or uid > 2**31
                or not 0 < len(name) <= 1024 or "\0" in name
                or identifier in producers):
            raise ValueError("Invalid producer identity")
        producers[identifier] = (name, uid)
    sources = set()
    for wire, value in top.get(2, []):
        if wire != 2:
            raise ValueError("Datasource protobuf wire mismatch")
        source = fields(value)
        identifier = one(source, 2, 0)
        descriptor = fields(one(source, 1, 2))
        name = one(descriptor, 1, 2).decode("utf-8")
        if (identifier not in producers or not 0 < len(name) <= 256
                or "\0" in name or (identifier, name) in sources):
            raise ValueError("Invalid datasource binding")
        sources.add((identifier, name))
    names = {name for _, name in sources}
    if not REQUIRED <= names:
        raise ValueError("Required trace sources absent")
    return dict(totalSessions=total, currentlyStartedSessions=started,
                producerCount=len(producers), dataSourceRows=len(sources),
                dataSourceNames=sorted(names))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_state", type=Path)
    args = parser.parse_args()
    try:
        with args.raw_state.open("rb") as source:
            raw = source.read(MAX_QUERY_BYTES + 1)
        decoded = service_state(raw)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps(decoded, sort_keys=True))


if __name__ == "__main__":
    main()
