"""Explicit API31 native state gate; retains raw SDK evidence before parsing."""
from pathlib import Path
import hashlib
import json
import subprocess
import rf312_instrumentation_state as state


class Gate:
    def __init__(self,adb,output):
        self.adb=adb;self.output=Path(output)/'instrumentation-state'
        self.output.mkdir(exist_ok=False);self.ordinal=0;self.proof=[]

    def sdk(self,argv,*,timeout):
        try:
            value=self.adb(*argv,timeout=timeout,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            return value.returncode,value.stdout,value.stderr
        except subprocess.CalledProcessError as error:return error.returncode,error.stdout,error.stderr

    def check(self,components,*,deadline,process_ids=None):
        self.ordinal+=1;prefix=self.output/str(self.ordinal).zfill(3)
        def record(row):
            for name in ['stdout','stderr']:prefix.with_suffix('.'+name).write_bytes(row[name])
            prefix.with_suffix('.command.json').write_text(json.dumps(dict(exitCode=row['exitCode'],
                command=['shell','dumpsys','activity','processes'],performanceSampleAccepted=False),indent=2)+'\n',encoding='utf-8')
        rows,digest=state.capture(self.sdk,deadline,record)
        result=state.expected(rows,components,process_ids=process_ids)
        result.update(stdoutSha256=digest,ordinal=self.ordinal)
        prefix.with_suffix('.proof.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
        self.proof.append(result)
        return result
