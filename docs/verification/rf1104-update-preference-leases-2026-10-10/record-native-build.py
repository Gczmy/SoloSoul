from pathlib import Path
import hashlib
import json

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
load = lambda name: json.loads((stage/name).read_text(encoding='utf-8'))
receipt = load('native-release-build.receipt.json')
assert receipt['exitCode'] == 0 and receipt['pdfiumOverrideInherited'] is False
expected = load('regression-green.sources.json')
manifest = (root/'tauri/src-tauri/Cargo.toml').read_bytes()
assert manifest == (stage/'release-Cargo-before.toml').read_bytes()
assert hashlib.sha256(manifest).hexdigest() == expected['tauri/src-tauri/Cargo.toml']
if not receipt['sourceUnchanged']:
    assert receipt['changedSourceFiles'] == ['tauri/src-tauri/Cargo.toml']
    parity = load('release-manifest-newline-parity.json')
    assert parity['normalizedBytesEqual'] and parity['parsedTomlEqual']
    assert parity['rawBuildReceiptSourceUnchanged'] is False
package = load('runtime-package.json')
assert package['windowsGuiSubsystem'] == 2 and len(package['resources']) == 94
assert package['sourceManifestSha256'] == hashlib.sha256((stage/'regression-green.sources.json').read_bytes()).hexdigest()
old = json.loads((root/'build/rf312-maintenance-20261009/runtime-package.json').read_text(encoding='utf-8'))
assert package['sha256'] != old['sha256']
assert {item['source']:item['sha256'] for item in package['resources']} == {
    item['source']:item['sha256'] for item in old['resources']}
assert load('native-auth-100.live.json')['command'][-2:] == ['--samples','3']
title = '### RF-1104 新 GUI Release 构建与原生回验启动（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- 非默认 native-perf GUI Release 构建取得 exit 0，Windows PE GUI subsystem=2；新 EXE SHA256 '+package['sha256']+'，与旧构建不同，94个运行资源与原冻结包逐字节一致。前端生产资产实际重新构建完成，未打包或发布。\n'
entry += '- Tauri CLI仅转换Cargo.toml换行：原始构建收据sourceUnchanged=false及前后原件保留；规范化字节和解析TOML均一致后只恢复原始换行。恢复后的manifest SHA与Rust冻结输入相同，流水线在原生行程前重新核验完整源码与运行包。\n'
entry += '- 已启动100对象三次完整原生行程，随后5000对象同口径三次；原公开夹具、旧失败样本及旧EXE保留。GUI环境不继承PDFIUM_LIBRARY_PATH测试覆盖，不调整128次调用/50秒阶段/300秒行程预算；尚无整组原生验收结论。\n'
entry += '- RF-1104继续[~]，284/294与其他前置不变；待六次原生合同核验、证据归档和独立本地提交，goal active。\n'
report.write_bytes(data+entry.replace('\n','\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','freshGuiBuildExitCode':0,'resourceFiles':94,
    'originalManifestRestored':True,'nativeGroup100Started':True,'taskClosed':False}))
