"""Run the same 206 related regressions plus this portable queue's regressions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

import queue_context as context

MODULES = (
    'test_rf312_instrumentation_state', 'test_rf312_state_gate',
    'test_rf312_default_memory', 'test_rf312_default_journey',
    'test_rf312_default_state_driver', 'test_rf312_default_binding',
    'test_rf312_journey_memory', 'test_android_process_memory',
    'test_diagnostic_exchange', 'test_rf312_memory_batches',
    'test_rf312_ime_readiness', 'test_rf312_owned_ui', 'test_persistent_adapter',
)


def collected():
    sys.path.insert(0, str(context.WORK / 'tauri/scripts'))
    return unittest.TestLoader().loadTestsFromNames(MODULES)


def queue_collected():
    return unittest.TestLoader().discover(str(context.ROOT), pattern='test_*.py',
                                         top_level_dir=str(context.ROOT))


def case_ids(suite):
    values = []
    for item in suite:
        values.extend(case_ids(item) if isinstance(item, unittest.TestSuite) else [item.id()])
    if len(set(values)) != len(values):
        raise ValueError('Unique regression case identifiers required')
    return values


def main():
    context.require_assertions()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    parent = context.ordinary(args.output.absolute().parent, directory=True)
    output = parent / args.output.name
    output.mkdir(exist_ok=False)
    temporary = output / 'temporary'
    temporary.mkdir(exist_ok=False)
    old_env = {key: os.environ.get(key) for key in ('TEMP', 'TMP')}
    old_temp = tempfile.tempdir
    os.environ.update(TEMP=str(temporary), TMP=str(temporary))
    tempfile.tempdir = None
    before = context.binding.sources()
    queue_before = context.sources()
    extra = {name: context.sha(context.WORK / name) for name in (
        'tauri/scripts/android-process-memory.py', 'tauri/scripts/test_android_process_memory.py')}
    try:
        suite = collected()
        queue_suite = queue_collected()
        ids, queue_ids = case_ids(suite), case_ids(queue_suite)
        assert len(before) == 97 and len(ids) == 206 and queue_ids
        with (output / 'tests.log').open('x', encoding='utf-8', newline='\n') as log:
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(suite)
            queue_result = unittest.TextTestRunner(stream=log, verbosity=2).run(queue_suite)
        assert context.binding.sources() == before and context.sources() == queue_before
        assert all(context.sha(context.WORK / name) == digest for name, digest in extra.items())
        receipt = dict(passed=result.wasSuccessful() and queue_result.wasSuccessful()
                       and not result.skipped and not queue_result.skipped,
                       tests=result.testsRun, testIds=ids, failures=len(result.failures),
                       errors=len(result.errors), skipped=len(result.skipped),
                       sourceSha256=before, externalReaderSourcesSha256=extra,
                       queueSourceSha256=queue_before, queueTests=queue_result.testsRun,
                       queueTestIds=queue_ids, queueTestsPassed=queue_result.wasSuccessful(),
                       queueSkipped=len(queue_result.skipped), logFile='tests.log',
                       logSha256=context.sha(output / 'tests.log'))
        with (output / 'result.json').open('x', encoding='utf-8', newline='\n') as stream:
            json.dump(receipt, stream, indent=2)
        print(json.dumps({key: receipt[key] for key in ('passed', 'tests', 'queueTests', 'skipped', 'queueSkipped')}))
        return 0 if receipt['passed'] else 1
    finally:
        tempfile.tempdir = old_temp
        for key, value in old_env.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        if not list(temporary.iterdir()):
            temporary.rmdir()


if __name__ == '__main__':
    sys.exit(main())
