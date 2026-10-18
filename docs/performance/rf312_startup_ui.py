"""Accept only a bound dump success, or the observed zero-exit null-root wait."""
import re
import xml.etree.ElementTree as ET

NULL_ROOT='ERROR: null root node returned by UiTestAutomationBridge.'


def dump_ready(result,path):
    if not isinstance(path,str) or re.fullmatch(r'/sdcard/rf312-ipc-[a-f0-9]{32}\.xml',path) is None:
        raise ValueError('Owned startup XML path required')
    if (type(result.returncode) is not int or not isinstance(result.stdout,bytes)
            or not isinstance(result.stderr,bytes)):
        raise ValueError('Exact SDK response types required')
    stdout=result.stdout.decode('utf-8').strip('\r\n')
    stderr=result.stderr.decode('utf-8').strip('\r\n')
    if result.returncode==0 and stdout=='' and stderr==NULL_ROOT:return False
    if result.returncode==0 and stderr=='' and stdout=='UI hierchary dumped to: '+path:return True
    raise ValueError('Unknown or unbound startup dump outcome')


def read_tree(call,path):
    result=call('startup-ui',['shell','uiautomator','dump',path],required=False,timeout=10)
    if not dump_ready(result,path):return None
    result=call('startup-ui-read',['exec-out','cat',path])
    if type(result.returncode) is not int or result.returncode!=0 or result.stderr!=b'' or not isinstance(result.stdout,bytes):
        raise ValueError('Startup hierarchy read failed')
    try:tree=ET.fromstring(result.stdout)
    except ET.ParseError as error:raise ValueError('Startup result is not complete XML') from error
    if tree.tag!='hierarchy':raise ValueError('Native accessibility hierarchy root required')
    return tree,result.stdout
