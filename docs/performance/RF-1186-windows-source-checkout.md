# RF-1186：Windows 检出保留冻结 Java 验证源件字节

真实 Windows Git 检出复现：对当前 97 份验证运行源件使用 `core.autocrlf=true`，五份 Java 辅助构建／绑定源件及回归变为 CRLF。原适配输入被 `Default runtime or adapter source changed` 拒绝，同一个原冻结 Java 包也被 `Exact source-bound Android UI helper package required` 拒绝。相同代码提交的检出换行改变了绑定所需的原始字节。

只补齐 `.gitattributes` 的 LF 契约，覆盖 `rf312_owned_ui.py`、对应测试，以及 `rf312-owned-ui` 中的 Java／Python 源件。原始源码内容、JAR、摘要接收规则、产品和运行预算保持。硬件配置不属于此缺陷：它已有 `text: unset` 保护，前后实际字节、摘要和原准备器输入准入均正常，因此没有修改它的契约。

修复后在另一个全新目录再次执行实际 Windows Git 检出，97 份源件的原字节及 SHA 全部与原索引一致；同一份原适配输入和原 Java 包的严格加载均通过。两个检查阶段各自核验原源件和索引未被检出命令覆盖，所有新输出均在专用 D 盘目录。校验的 Python 审计钩子拒绝子进程启动，实际 SDK 启动数为零。

这是检出字节和冻结输入／制品的准入验证，不是原生界面或性能样本。此前 206 项相关回归绑定的 97 份运行代码保持原字节。改动为可逆的 Git 属性，未新增复述属性内容的测试，使用真实检出及原严格加载器的修复前拒绝／修复后通过验收。新公开队列的独立来源另行发布与验收，不计入本项的 97 份范围。

见[去标识摘要](rf1186-windows-source-checkout-2026-10-18.json)。RF-312 的重复分布、应用本体、启动两帧、实际 IPC、观察成本及多端仍未完成；本项不关闭父任务。
