import pathlib,subprocess,os,time,json
root=pathlib.Path('/Users/zzc/PycharmProjects/SoloSoul/tauri');out=pathlib.Path('/tmp/solosoul-refactor-macos-20261002');manifest=root/'src-tauri/Cargo.toml';original=manifest.read_bytes();old=b'tauri = { version = "2", features = [] }';assert original.count(old)==1;temporary=original.replace(old,b'tauri = { version = "2", features = ["macos-private-api"] }',1);env=os.environ.copy();env['PATH']='/Users/zzc/.nvm/versions/node/v24.14.0/bin:/Users/zzc/.rustup/toolchains/stable-aarch64-apple-darwin/bin:'+env['PATH'];env['PDFIUM_LIBRARY_PATH']=str(root/'src-tauri/resources/pdfium/libpdfium.dylib');args=['cargo','run','-p','solo_soul','--example','macos_window_appearance','--','--unthrottled','--card-fixture',str(out/'rf121-card-surfaces-diagnostic.html')];start=time.monotonic()
try:
 manifest.write_bytes(temporary)
 with (out/'rf121-card-surfaces-native.log').open('w') as f:code=subprocess.call(args,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 (out/'rf121-card-surfaces-native-result.json').write_text(json.dumps(dict(command=args,exit_code=code,duration_seconds=time.monotonic()-start,temporary_macos_private_feature=True),indent=2)+'\n');print('macOS standalone window',code,flush=True)
finally:
 assert manifest.read_bytes()==temporary;manifest.write_bytes(original)
