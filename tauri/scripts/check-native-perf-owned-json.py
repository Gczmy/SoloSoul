"""Require real Windows reader tests, including a native file symlink, with no skips."""
import argparse
import ctypes
from ctypes import wintypes as w
import hashlib
import json
import os
from pathlib import Path
import platform
import tempfile
import unittest


SYMLINK_TEST = "test_final_file_symlink_is_rejected_before_content_read"


def cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from cases(item)
        else:
            yield item


def fixture_path_facts(tests):
    """Report alias classes without publishing temporary paths or user names."""
    facts = {
        "reportedFixtureRoots": sum(hasattr(test, "fixture_root_canonicalized") for test in tests),
        "canonicalizedFixtureRoots": sum(getattr(test, "fixture_root_canonicalized", False) for test in tests),
        "noncanonicalAliasesRejectedBeforeRead": sum(getattr(test, "noncanonical_fixture_alias_rejected", False) for test in tests),
    }
    if os.name != "nt":
        return facts
    requested = Path(tempfile.gettempdir())
    canonical = requested.resolve(strict=True)
    api = ctypes.WinDLL("kernel32", use_last_error=True)
    api.GetLongPathNameW.argtypes = [w.LPCWSTR, w.LPWSTR, w.DWORD]
    api.GetLongPathNameW.restype = w.DWORD
    expanded = ctypes.create_unicode_buffer(32768)
    count = api.GetLongPathNameW(str(requested), expanded, len(expanded))
    if not count:
        raise ctypes.WinError(ctypes.get_last_error())
    if count >= len(expanded):
        raise ValueError("Temporary root expansion exceeds its path limit")
    def normalize(path):
        return os.path.normcase(os.fspath(path))

    facts.update({
        "temporaryRootSpellingChanged": normalize(requested) != normalize(canonical),
        "temporaryRootSameDirectory": os.path.samefile(requested, canonical),
        "shortNameExpansionChanged": normalize(requested) != normalize(expanded.value),
        "expandedRootMatchesCanonical": normalize(expanded.value) == normalize(canonical),
    })
    return facts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="Write a new JSON receipt.")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    suite = unittest.defaultTestLoader.discover(
        str(root), pattern="test_native_perf_owned_json.py"
    )
    tests = list(cases(suite))
    symlink_tests = [test for test in tests if test.id().endswith("." + SYMLINK_TEST)]
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    refused = {test.id() for test, _ in result.failures + result.errors + result.skipped}
    symlink = symlink_tests[0] if len(symlink_tests) == 1 else None
    tag = getattr(symlink, "symlink_reparse_tag", None)
    content_reads = getattr(symlink, "symlink_content_read_calls", None)
    target_preserved = getattr(symlink, "symlink_target_preserved", False)
    native_symlink_accepted = (
        symlink is not None
        and symlink.id() not in refused
        and tag == 0xA000000C
        and content_reads == 0
        and target_preserved is True
    )
    accepted = (
        os.name == "nt"
        and len(tests) >= 11
        and result.testsRun == len(tests)
        and result.wasSuccessful()
        and not result.skipped
        and native_symlink_accepted
    )
    receipt = {
        "schemaVersion": 1,
        "task": "RF-1128",
        "accepted": accepted,
        "nativeWindows": os.name == "nt",
        "pythonVersion": platform.python_version(),
        "readerSha256": hashlib.sha256(
            (root / "native-perf-owned-json.py").read_bytes()
        ).hexdigest(),
        "testSha256": hashlib.sha256(
            (root / "test_native_perf_owned_json.py").read_bytes()
        ).hexdigest(),
        "discoveredTests": len(tests),
        "testsRun": result.testsRun,
        "failures": [test.id() for test, _ in result.failures],
        "errors": [test.id() for test, _ in result.errors],
        "skipped": [{"test": test.id(), "reason": reason} for test, reason in result.skipped],
        "fixturePaths": fixture_path_facts(tests),
        "fileSymlink": {
            "accepted": native_symlink_accepted,
            "reparseTag": f"0x{tag:08x}" if isinstance(tag, int) else None,
            "contentReadCalls": content_reads,
            "targetPreserved": target_preserved,
        },
    }
    if args.output:
        with args.output.open("x", encoding="utf-8", newline="\n") as output:
            json.dump(receipt, output, indent=2)
            output.write("\n")
    print(json.dumps(receipt, indent=2))
    return 0 if accepted else 1


if __name__ == "__main__":
    raise SystemExit(main())
