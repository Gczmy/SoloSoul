from pathlib import Path
p = Path('D:/SoloSoul/docs/REFACTOR_EXECUTION_REPORT_2026-09-25.md')
text = p.read_bytes().decode('utf-8')
newline = '\r\n' if '\r\n' in text else '\n'
assert '### RF-1104' not in text
text = text.replace('> 最后更新：2026-10-09（RF-312 维护准入诊断完成本轮集成与原生复跑，整项待验证）',
    '> 最后更新：2026-10-10（RF-1104 更新源偏好许可范围修复进行中；RF-312 整项待验证）')
text = text.replace('> 当前分支：`codex/rf312-maintenance-admission`', '> 当前分支：`codex/rf1104-update-preference-leases`')
lines = text.splitlines()
for i, line in enumerate(lines):
    if line.startswith('- 任务总数：**293**'):
        lines[i] = '- 任务总数：**294**（P1：55；P2：238；P3：1）。'
    elif line.startswith('- 已关闭：**284 / 293**'):
        lines[i] = line.replace('284 / 293', '284 / 294').replace('进行中：0', '进行中：1').replace('RF-1102、RF-1103（', 'RF-1102、RF-1103、RF-1104（')
    elif line.startswith('- 当前处理：**RF-312 本轮'):
        lines[i] = '- 当前处理：**RF-1104：网络选源不持有目录许可，结果返回后验证原目录与会话并保护实际写入。RF-112/121/312 与 RF-122～127 前置保持**。'
text = newline.join(lines)+newline
row = '| 294 | [RF-1104](#rf-1104) | P1 | 更新源网络等待不阻断重解锁，写入重新校验原目录与会话 | 无（RF-312维护准入诊断发现） | [~] 进行中 |'
text = text.replace(newline+'## 5. 原报告到执行任务的映射', row+newline+newline+'## 5. 原报告到执行任务的映射',1)
card = '''### RF-1104

**更新源网络等待不阻断重解锁，写入重新校验原目录与会话** · P1 · 来源：RF-312 维护准入诊断

- **入口：**`tauri/src-tauri/src/commands/update_preferences.rs`；两个调用入口为桌面清单检查和共用 Release 选源。
- **现状证据：**主分支 `32b771e33` 中 `SourcePreferences` 持有实际目录许可与账户 Store，两个入口将其保留到网络响应之后。此前100对象第3次、5000对象第2次重解锁均返回 `operations-active`，存在覆盖准入区间的更新源偏好许可；本项建立真实生产选源与维护准入的定向回归，不将此错误等同于旧首页超时根因。
- **执行：**读取期间持有实际许可，网络等待只保留偏好与原身份快照；保存时重新获得许可、核对原 root owner/Store/账户/会话并在会话提交门闩内进行同步写入。维护忙、锁定/重解锁、换账户/目录或原服务释放时拒绝迟到持久化，不能回退到无许可的缓存写入。不修改 Core 互斥、SDK 预算或真实 worker 生命周期。
- **验收：**延迟网络不阻断真实重解锁；正常账户与登录前缓存持久化、无变化版本优化、双通道按键合并保持；维护忙/同账户新会话/换账户/换根/原 owner 释放的迟到结果不写入；实际文件写入期间维护仍被拒绝，写完才释放许可。
- **验证配置：**真实红绿回归、默认 R、非默认 Windows 严格 Clippy/许可与诊断回归、DOC；随后以新构建独立复跑100/5000各三次完整原生行程，整组失败保留，RF-312 不随此修复提前关闭。

'''
text = text.replace('## 7. 每项执行记录模板', card.replace('\n',newline)+'## 7. 每项执行记录模板',1)
p.write_bytes(text.encode('utf-8'))
print('Registered RF-1104: 294 tasks, 284 closed, 1 in progress, 9 other open')
