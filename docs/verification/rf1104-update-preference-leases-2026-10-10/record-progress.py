from pathlib import Path
import json

root = Path('D:/SoloSoul')
stage = Path(__file__).parent
report = root/'docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md'
initial = json.loads((stage/'regression-red.receipt.json').read_text())
red = json.loads((stage/'regression-red-fixed.receipt.json').read_text())
assert initial['exitCode'] == red['exitCode'] == 101
assert initial['sourceUnchanged'] and red['sourceUnchanged']
assert 'IMPORT_OPERATIONS_ACTIVE' in (stage/'regression-red-fixed.stderr.log').read_text(encoding='utf-8')
text = report.read_text(encoding='utf-8')
assert '### RF-1104 执行记录' not in text
text += '''

### RF-1104 执行记录（2026-10-10，进行中）

- 首轮新增回归因密码参数类型错误未通过编译，exit 101；修正为现有 Zeroizing<String> 接口后，生产实现与主分支逐字规范化比对一致，实际运行 1 项回归得到 0 passed / 1 failed，维护准入返回 IMPORT_OPERATIONS_ACTIVE。两轮原始日志、退出码和检查前后不变的源码 SHA 均保留，不能把首轮编译失败当作缺陷红测。
- 已将 SourcePreferences 的账户 Store、服务和 root owner 改为弱引用身份快照；读取和实际保存分别持有活动许可，网络选源不持有许可。保存重新核对原目录、账户、会话代次与 Store，并在现有会话提交门闩内同步写入。Core 互斥规则、网络选源算法、SDK 预算和生产依赖未改。
- 保留正常持久化、双通道合并、无变化版本优化及登录前公共缓存；迁移旧的许可保护用例，并补同账户新会话、账户切换、目录替换/同路径新 owner、服务释放、并发双通道和实际文件写入保护。旧 Store 保持存活的同账户重解锁也须拒绝迟到结果。
- 修复后定向回归已启动，尚未获得终态；默认 R、非默认诊断/许可回归和新构建两档各三次原生行程仍待执行。本项继续 [~]，284/294、3 待验证、6 待执行保持；尚未提交或推送，不提前关闭 RF-312 或解除 RF-112/121/122～127 前置，goal active。
'''
report.write_bytes(text.replace('\n','\r\n').encode('utf-8'))
print('Recorded genuine red result and pending RF-1104 validation')
