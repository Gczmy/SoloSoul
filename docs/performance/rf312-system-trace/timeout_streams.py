"""Preserve only actually available timeout bytes; never invent empty streams."""
from pathlib import Path
import hashlib,json,subprocess


def preserve_timeout(prefix, error):
    if not isinstance(error, subprocess.TimeoutExpired):
        raise ValueError("Expected TimeoutExpired")
    prefix = Path(prefix)
    streams = {}
    for name in ("stdout", "stderr"):
        value = getattr(error, name)
        if value is not None and not isinstance(value, bytes):
            raise ValueError("Timeout stream must be raw bytes or unavailable")
        streams[name] = dict(available=value is not None,
                             bytes=None if value is None else len(value),
                             sha256=None if value is None else hashlib.sha256(value).hexdigest())
    receipt = dict(timedOut=True, exitCode=None, timeoutSeconds=error.timeout,
                   command=error.cmd, partialObservedOnly=True, streams=streams)
    # Reserve the receipt first so a duplicate cannot overwrite earlier bytes.
    with prefix.with_suffix(".timeout.json").open("x", encoding="utf-8") as output:
        for name in ("stdout", "stderr"):
            value = getattr(error, name)
            if value is not None:
                with prefix.with_suffix(".partial." + name).open("xb") as raw:
                    raw.write(value)
        json.dump(receipt, output, indent=2)
        output.write("\n")
    return receipt
