"""Recognize only observed SDK bootstrap clients with a live, exact owned emulator ancestry."""
from pathlib import PureWindowsPath
import datetime,os,re
FIELDS={'Name','ProcessId','ParentProcessId','CreationDate','ExecutablePath','CommandLine'}
def birth(row):
 value=row['CreationDate']
 if not isinstance(value,str):raise ValueError('OS process birth required')
 parsed=datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
 if parsed.tzinfo is None:raise ValueError('Offset-aware birth required')
 return parsed
def valid_id(value):return type(value) is int and 0<value<2**32
def same_root(actual,expected):
 return (set(expected)==FIELDS and all(actual.get(k)==expected[k] for k in ['Name','ProcessId','ParentProcessId','CommandLine'])
  and PureWindowsPath(actual['ExecutablePath'])==PureWindowsPath(expected['ExecutablePath']) and birth(actual)==birth(expected))
def is_owned_sdk_client(process,rows,emulator_root,sdk_root,avd,serial):
 try:
  if not re.fullmatch(r'SoloSoul_[A-Za-z0-9_.-]{1,96}',avd) or serial!='emulator-5578':return False
  sdk=PureWindowsPath(str(sdk_root));adb=sdk/'platform-tools/adb.exe';emulator=sdk/'emulator/emulator.exe'
  if not sdk.is_absolute() or set(process)!=FIELDS or not valid_id(process['ProcessId']):return False
  if process['Name']!='adb.exe' or PureWindowsPath(process['ExecutablePath'])!=adb:return False
  command=process['CommandLine'];tail=None
  for prefix in [str(adb),'"'+str(adb)+'"']:
   if command[:len(prefix)].casefold()==prefix.casefold() and command[len(prefix):].startswith(' '):tail=' '.join(command[len(prefix):].split());break
  allowed={
   '-s '+serial+' shell settings put system screen_off_timeout 2147483647',
   '-s '+serial+' shell settings put system accelerometer_rotation 1',
   '-s '+serial+' shell am broadcast -a com.android.emulator.multidisplay.START -n com.android.emulator.multidisplay/.MultiDisplayServiceReceiver "--user 0"',
  }
  if tail not in allowed:return False
  graph={}
  for row in rows:
   pid=row['ProcessId']
   if type(pid) is not int or not 0<=pid<2**32:return False
   if pid==0:continue
   if pid in graph:return False
   graph[pid]=row
  root=graph.get(emulator_root['ProcessId'])
  if root is None or not same_root(root,emulator_root) or root['Name']!='emulator.exe' or PureWindowsPath(root['ExecutablePath'])!=emulator:return False
  if not re.search(r'(?:^|\s)-avd\s+"?'+re.escape(avd)+r'"?(?:\s|$)',root['CommandLine']):return False
  root_birth=birth(root);previous=birth(process);current=process['ParentProcessId'];seen=set();qemu_seen=False
  for _ in range(16):
   if not valid_id(current) or current in seen or current not in graph:return False
   seen.add(current);parent=graph[current];when=birth(parent)
   if not root_birth<=when<=previous:return False
   if current==root['ProcessId']:return qemu_seen
   exe=PureWindowsPath(parent['ExecutablePath'])
   if parent['Name'].startswith('qemu-system-'):
    if not exe.is_relative_to(sdk/'emulator/qemu') or exe.name.casefold()!=parent['Name'].casefold():return False
    if not re.search(r'(?:^|\s)-avd\s+"?'+re.escape(avd)+r'"?(?:\s|$)',parent['CommandLine']):return False
    qemu_seen=True
   elif parent['Name']=='cmd.exe':
    if exe!=PureWindowsPath(os.environ.get('SystemRoot',r'C:\Windows'))/'System32/cmd.exe':return False
   else:return False
   previous=when;current=parent['ParentProcessId']
  return False
 except (KeyError,TypeError,ValueError,AttributeError):return False
