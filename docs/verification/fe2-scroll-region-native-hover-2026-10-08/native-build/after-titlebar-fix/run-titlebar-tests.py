import hashlib, json, os, subprocess, time
from pathlib import Path
OUT=Path('/tmp/solosoul-fe2-macos-scroll-hover-build-20261008-titlebar')
WORKSPACE=Path('/tmp/solosoul-fe2-macos-scroll-hover-build-20261008/workspace/tauri')
assert json.loads((OUT/'report.json').read_text())['passed'], 'Build must complete successfully before tests'
assert WORKSPACE.resolve()!=Path('/Users/zzc/PycharmProjects/SoloSoul/tauri').resolve()
env=os.environ.copy()
env['PATH']='/Users/zzc/.nvm/versions/node/v24.14.0/bin:/Users/zzc/.rustup/toolchains/stable-aarch64-apple-darwin/bin:'+env['PATH']
env.update({'CARGO_NET_OFFLINE':'true', 'CARGO_TARGET_DIR':str(WORKSPACE/'target'), 'ORT_LIB_PATH':'/Users/zzc/Library/Caches/ort.pyke.io/dfbin/aarch64-apple-darwin/612739f75438dc0a075461e1fb454226b4a1eb175e60a7271ba966bbbb972cd4'})
command=['cargo','test','-p','solo_soul','--lib','commands::window::macos::titlebar::tests']
report={'command':command,'cwd':str(WORKSPACE),'target':str(WORKSPACE/'target'),'started_at':time.time(),'passed':False,'account_tests_requested':False,'window_launched':False}
with (OUT/'titlebar-tests.log').open('w') as log:
    result=subprocess.run(command,cwd=WORKSPACE,env=env,stdout=log,stderr=subprocess.STDOUT)
report.update({'exit_code':result.returncode,'passed':result.returncode==0,'finished_at':time.time()})
text=(OUT/'titlebar-tests.log').read_text()
report['two_requested_tests_passed']=all(name+' ... ok' in text for name in ['commands::window::macos::titlebar::tests::pointer_tracking_events_pass_through_titlebar','commands::window::macos::titlebar::tests::mouse_buttons_and_dragging_keep_native_titlebar_target'])
report['passed']=report['passed'] and report['two_requested_tests_passed']
(OUT/'titlebar-tests-result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False,indent=2),flush=True)
raise SystemExit(0 if report['passed'] else 1)
