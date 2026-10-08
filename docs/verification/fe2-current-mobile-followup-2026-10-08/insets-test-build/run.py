import hashlib,json,os,shutil,stat,subprocess,time
from pathlib import Path
ROOT=Path('/Users/zzc/PycharmProjects/SoloSoul'); OUT=Path('/tmp/solosoul-fe2-android-current-api31-insets-final-test-build-20261008')
paths=['tauri/Cargo.toml','tauri/src-tauri/Cargo.toml','tauri/src-tauri/gen/apple/project.yml','tauri/src-tauri/gen/apple/solo_soul.xcodeproj','tauri/src-tauri/gen/apple/solo_soul_iOS/Info.plist','tauri/src-tauri/gen/apple/assets','tauri/src-tauri/gen/android/app/src/main/assets','tauri/src-tauri/gen/android/app/tauri.properties']
def inventory(root):
 if not root.exists() and not root.is_symlink():return None
 result={}
 for p in [root]+(sorted(root.rglob('*')) if root.is_dir() and not root.is_symlink() else []):
  key='.' if p==root else str(p.relative_to(root));mode=stat.S_IMODE(p.lstat().st_mode)
  if p.is_symlink():row={'type':'symlink','mode':mode,'target':os.readlink(p)}
  elif p.is_dir():row={'type':'directory','mode':mode}
  elif p.is_file():row={'type':'file','mode':mode,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
  else:raise RuntimeError('Unexpected input type')
  result[key]=row
 return result
def remove(p):
 if p.is_dir() and not p.is_symlink():shutil.rmtree(p)
 elif p.exists() or p.is_symlink():p.unlink()
def copy(src,dst):
 dst.parent.mkdir(parents=True,exist_ok=True)
 if src.is_symlink():dst.symlink_to(os.readlink(src))
 elif src.is_dir():shutil.copytree(src,dst,symlinks=True)
 else:shutil.copy2(src,dst)
checkpoint=json.loads((ROOT/'docs/verification/fe2-frontend-checkpoint-2026-10-07.json').read_text())
sources={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in checkpoint['source_sha256']}
(OUT/'source.json').write_text(json.dumps({'recorded_at':time.time(),'source_sha256':sources},indent=2)+'\n')
backups={}; report={'scope':'Current source ARM64 Android instrumentation APK; production APK separately frozen','passed':False,'started_at':time.time()}
try:
 for i,name in enumerate(paths):
  src=ROOT/name;inv=inventory(src);dst=OUT/'protected-backups'/str(i)
  if inv is not None:copy(src,dst);assert inventory(dst)==inv
  backups[name]=(dst,inv)
 print('Protected input snapshots verified',len(backups),flush=True)
 env=os.environ.copy();env['PATH']='/Users/zzc/.nvm/versions/node/v24.14.0/bin:/Users/zzc/.rustup/toolchains/stable-aarch64-apple-darwin/bin:'+env['PATH']
 env.update({'JAVA_HOME':'/Applications/Android Studio.app/Contents/jbr/Contents/Home','ANDROID_HOME':'/Users/zzc/Library/Android/sdk','ANDROID_NDK_HOME':'/Users/zzc/Library/Android/sdk/ndk/30.0.14904198'})
 args=['./gradlew',':app:assembleArm64DebugAndroidTest','-x',':app:rustBuildArm64Debug','--offline','--no-daemon']
 report['command']=args
 with (OUT/'build.log').open('w') as f:r=subprocess.run(args,cwd=ROOT/'tauri/src-tauri/gen/android',env=env,stdout=f,stderr=subprocess.STDOUT)
 report['exit_code']=r.returncode
 assert r.returncode==0,'Current Android build failed'
 apk=ROOT/'tauri/src-tauri/gen/android/app/build/outputs/apk/androidTest/arm64/debug/app-arm64-debug-androidTest.apk'
 assert apk.is_file()
 report['apk']=str(apk);report['apk_sha256']=hashlib.sha256(apk.read_bytes()).hexdigest()
 frozen=OUT/'app-arm64-debug-androidTest.apk';shutil.copy2(apk,frozen)
 assert hashlib.sha256(frozen.read_bytes()).hexdigest()==report['apk_sha256']
 report['passed']=True
 report['frozen_apk']=str(frozen);report['passed']=True
except Exception as e:report['error']=str(e);print('ERROR',e,flush=True)
finally:
 restored={}
 for name,(backup,inv) in backups.items():
  dest=ROOT/name
  if inventory(dest)!=inv:
   remove(dest)
   if inv is not None:copy(backup,dest)
  restored[name]=inventory(dest)==inv
 report['protected_inputs_restored']=restored
 after={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in sources}
 report['source_differences_after_restore']=[p for p,h in sources.items() if after[p]!=h]
 if not all(restored.values()) or report['source_differences_after_restore']:report['passed']=False
 report['finished_at']=time.time();(OUT/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:report.get(k) for k in ('passed','exit_code','error','apk_sha256','source_differences_after_restore')},ensure_ascii=False),flush=True)
raise SystemExit(0 if report['passed'] else 1)
