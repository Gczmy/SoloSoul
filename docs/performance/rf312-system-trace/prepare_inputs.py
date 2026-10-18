"""Prepare a fresh portable transport directory; never starts an SDK or device."""
import argparse
import hashlib
import json
from pathlib import Path
import uuid

HERE = Path(__file__).resolve().parent
PAYLOAD = {
    "transport.py": "c3fdffb062ca9ee468cced9c64eed23705fa37813d83f20eb9096cf4ac55d6ad",
    "timeout_streams.py": "da54d225722d2b674b6b8fcef022ba129b7c53ff5be969e7afa3a5990c4e4a72",
    "base-config.pbtxt": "de6be90adb296dcaa2cb168e141fb76d5eb6ffe0ed15340c174bab544d40a518",
}


def prepare(destination):
    destination = Path(destination).absolute()
    if destination.exists() or destination.is_symlink():
        raise ValueError("Destination must not exist")
    destination = destination.resolve()
    if not destination.parent.is_dir() or destination.is_relative_to(HERE.resolve()):
        raise ValueError("Use a new directory outside the source directory with an existing parent")
    contents = {}
    for name, expected in PAYLOAD.items():
        source = HERE / name
        if source.is_symlink():
            raise ValueError("Source must not be a symbolic link")
        data = source.read_bytes()
        if hashlib.sha256(data).hexdigest() != expected:
            raise ValueError("Source binding changed: " + name)
        contents[name] = data
    nonce = uuid.uuid4().hex
    output = "/data/misc/perfetto-traces/rf312-public-" + nonce + "-output.trace"
    contents["file-plan.json"] = (json.dumps({"nonce": nonce}) + "\n").encode("ascii")
    contents["trace-config.pbtxt"] = contents["base-config.pbtxt"] + (
        '\nwrite_into_file: true\noutput_path: "' + output + '"\n').encode("ascii")
    contents["roundtrip.bin"] = b"RF312-input-roundtrip\x00" + bytes(range(256)) + b"\r\n"
    destination.mkdir()
    for name, data in contents.items():
        with (destination / name).open("xb") as stream:
            stream.write(data)
    receipt = dict(schemaVersion=1, scope="five-second-platform-trace-input-preparation-only",
                   sourceFiles=PAYLOAD,
                   frozenInputs={name: hashlib.sha256(data).hexdigest()
                                 for name, data in sorted(contents.items())},
                   sdkStarted=False, deviceAdmitted=False,
                   nativeFramesAccepted=False, fullRF312Accepted=False)
    with (destination / "prepared-inputs.json").open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(receipt, stream, indent=2)
        stream.write("\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    try:
        receipt = prepare(args.destination)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()
