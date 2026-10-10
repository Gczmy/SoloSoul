from pathlib import Path
import hashlib
import json

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
proof = json.loads((stage/'rust-validation.json').read_text(encoding='utf-8'))
assert proof['sourceFileCount'] == 1827
assert proof['sourcesManifestSha256'] == hashlib.sha256((stage/'regression-green.sources.json').read_bytes()).hexdigest()
assert len(proof['checks']) == 8
for name, value in proof['checks'].items():
    receipt = json.loads((stage/(name+'.receipt.json')).read_text(encoding='utf-8'))
    assert receipt == value['receipt']
    assert receipt['exitCode'] == 0 and receipt['sourceUnchanged']
assert proof['checks']['rust-tests-default']['testTotals'] == {'passed':1871,'failed':0,'ignored':3}
for name, count in [('rust-tests-native',113),('rust-tests-rf905',29),('rust-tests-preferences-native',15)]:
    assert proof['checks'][name]['testTotals'] == {'passed':count,'failed':0,'ignored':0}
title = '### RF-1104 全部必需 Rust 检查通过（2026-10-10）'
data = report.read_bytes()
assert title.encode('utf-8') not in data
entry = '\n\n'+title+'\n\n'
entry += '- 八份检查收据、真实测试输出与同一1827文件冻结输入已统一核验：定向绿测15；格式、默认与非默认严格Clippy exit 0；默认workspace1871 passed / 0 failed / 3既有ignored；非默认诊断113、RF-905许可29、非默认完整偏好15，均0 failed / 0 ignored / exit 0。默认完整运行和非默认偏好运行均包含同一15项回归，三项既有ignored名称与旧基线一致。\n'
entry += '- 新GUI Release构建已启动，随后使用原公开夹具独立执行100/5000对象各三次完整原生行程，运行环境不继承测试用PDFIUM_LIBRARY_PATH。原失败样本和构建保持；不得用Rust通过代替GUI或原生认证验收。\n'
entry += '- RF-1104仍[~]，284/294及RF-112/121/312和RF-122～127前置保持；完整原生验收、证据归档和本项独立提交仍待完成，尚未推送，goal active。\n'
report.write_bytes(data+entry.replace('\n','\r\n').encode('utf-8'))
print(json.dumps({'task':'RF-1104','requiredRustChecksAccepted':8,'sourceFiles':1827,
    'freshNativeAcceptancePending':True,'taskClosed':False}))
