"""Run and bind the complete local, offline public runtime regression suite."""
import argparse
import io
import json
from pathlib import Path
import sys
import unittest
from runtime_inputs import digest

S = Path(__file__).resolve().parent


def sources():
    return {p.name:digest(p) for p in sorted(S.iterdir()) if p.is_file()
            and (p.suffix in ('.py','.ps1','.sh','.java') or p.name in ('AndroidManifest.xml','api31-hardware.ini'))}


def leaves(suite):
    for item in suite:
        if isinstance(item,unittest.TestSuite):yield from leaves(item)
        else:yield item


def collected():
    suite=unittest.defaultTestLoader.discover(str(S),pattern='test_*.py')
    ids=[item.id() for item in leaves(suite)]
    if not ids or len(ids)!=len(set(ids)):raise ValueError('Nonempty unique complete test collection required')
    return suite,ids


def validate(result):
    _,ids=collected()
    if (result.get('passed') is not True or result.get('testIds')!=ids or result.get('tests')!=len(ids)
            or result.get('uniqueCases')!=len(ids) or result.get('skipped')!=0 or result.get('sourceSha256')!=sources()):
        raise ValueError('Passing current-source complete offline regression evidence required')


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args(argv);args.output.mkdir(exist_ok=False)
    before=sources();suite,ids=collected();stream=io.StringIO()
    result=unittest.TextTestRunner(stream=stream,verbosity=2).run(suite)
    text=stream.getvalue();(args.output/'tests.log').write_text(text,encoding='utf-8')
    value=dict(passed=result.wasSuccessful() and not result.skipped and sources()==before and result.testsRun==len(ids),
        tests=result.testsRun,uniqueCases=len(ids),testIds=ids,skipped=len(result.skipped),sourceSha256=before,
        logSha256=digest(args.output/'tests.log'),sdkCommandsExecuted=0,targetInputsDispatched=0)
    (args.output/'result.json').write_text(json.dumps(value,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:value[k] for k in ('passed','tests','uniqueCases','skipped')}))
    return 0 if value['passed'] else 1


if __name__=='__main__':sys.exit(main())
