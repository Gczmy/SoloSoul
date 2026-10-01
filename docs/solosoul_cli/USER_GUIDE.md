# SoloSoul CLI 用户手册

> 本文档面向 SoloSoul 终端用户（CLI TUI 客户端）。
> 适用版本：v2.3.x（unlock/lock/list/open/sync/ocr/embed_model 等全部命令）。

## 1. 安装与启动

| 平台 | 安装来源 |
|------|----------|
| macOS / Linux | `cargo install solosoul-cli`（Cargo crates）或源码编译 |
| Windows | MSI 安装包中的 `solosoul.exe` |

CLI 默认数据目录：
- Unix：`~/.solosoul/`
- Windows：`%USERPROFILE%\.solosoul\`

通过环境变量 `SOLOSOUL_DATA_DIR` 可重定向到任意目录（用于测试或多账户隔离）。

桌面 CLI 在启动前取得 Native root 所有权；同一 canonical root 已被 GUI 或另一个 CLI 使用时拒绝启动，具体生命周期与平台边界见 §5。

## 2. 启动

```bash
solosoul                  # 启动 TUI 主页（或 Welcome 页）
solosoul --data-dir <DIR> # 指定数据目录
SOLOSOUL_DATA_DIR=<DIR> solosoul
```

CLI 启动时：
- 无本地账户 → 进入 **Welcome** 页面，按 Enter 进入创建账户向导。
- 有账户但未解锁 → **Locked** 页面，可执行 `/unlock`、`/account_list`、`/doctor`、`/exit`。
- 已解锁 → **Home** 页，显示 6 张快捷卡片（对象、搜索、模板、附件、备份、OCR 等）。

## 3. 全局快捷键

| 键 | 行为 |
|----|------|
| `↑` / `↓` | 命令历史 / 菜单上下选择 |
| `Tab` | 命令补全 |
| `Enter` | 执行命令或确认输入 |
| `Esc` | 退出向导 / 清空输入 / 关闭错误弹窗 |
| `Ctrl+L` | 手动锁定 Vault |
| `Ctrl+C` | 强制退出 CLI |
| 鼠标左键 | 单击快捷卡片 / 命令按钮 |
| 鼠标滚轮 | 滚动列表 |
| 鼠标拖拽 | 卡片/Lock 页内滚动 |

## 4. 命令清单

### 4.1 账户与解锁

| 命令 | 说明 |
|------|------|
| `/unlock` 或 `/login` | 进入登录向导（单账户直接跳到密码页） |
| `/lock` 或 `/logout` | 立即锁定 Vault |
| `/account_list` | 显示本地账户列表 |
| `/doctor` | 运行环境诊断（Vault 状态、进程锁、模型完整性、依赖等） |
| `/exit` | 退出 CLI |
| `/back` | 返回上一屏 |

### 4.2 数据对象

| 命令 | 说明 |
|------|------|
| `/list` | 显示当前账户的所有对象（页面+独立对象） |
| `/open <id>` | 打开对象详情页 |
| `/newpage` | 创建新页面 |
| `/newobject` | 进入创建对象向导 |
| `/edit <id>` | 编辑对象字段（向导） |
| `/delete <id>` | 软删除对象（进入回收站） |
| `/search <query>` | 在解密字段中流式搜索（命中 200 条截断） |
| `/size` / `/status` / `/state` | 显示账户统计报告 |

使用模板创建对象时，CLI 会保留字段定义、显式敏感度标签、模板名称、契约 ID 和模板指纹，并把字段定义与标签写入初始历史快照。之后删除模板也不会丢失这些副本；你填写的字段值和选择的页面、图标保持不变。这些内部元数据不会作为普通字段出现在详情或编辑列表。未使用模板时仍按原流程创建（RF-009）。GUI 与 CLI 已共用模板初始化和记录构造规则（RF-010）；CLI 的默认页面、名称、图标和向导交互保持原样。

### 4.3 回收站

| 命令 | 说明 |
|------|------|
| `/trash` 或 `/bin` | 回收站列表（键盘 `r` 恢复、`p` 永久删除、空格多选） |
| `/restore <id>` | 恢复单个对象 |
| `/purge <id>` | 永久删除单个对象 |

### 4.4 历史与审计

| 命令 | 说明 |
|------|------|
| `/history <id>` | 对象快照历史 |
| `/rollback <id> <ver>` | 回滚到指定快照版本 |
| `/operation_log [account_id]` | 显示审计日志 |
| `/export_log <path>` | 导出审计日志 |
| `/debug_log` | 导出诊断包（脱敏） |

CLI 回滚同时恢复字段敏感度标签，并把标签写入新生成的回滚快照，连续回滚不会丢失标签。快照中的 `propertyLabels` 优先于旧格式 `property_labels`；两者均缺失时保留对象当前标签，显式 `null` 清除标签，空对象 `{}` 恢复为空标签表。标签不是对象或 `null` 时，在写入对象、历史或审计前拒绝回滚。字段定义随快照的 `properties.__fields` 恢复，不依赖模板仍然存在；跨对象快照仍拒绝应用（RF-007）。

RF-008：CLI 与 GUI 共用回滚用例。对象保存后分别尝试回滚历史和审计；历史失败仍尝试记录审计。若任一后续步骤失败，CLI 明确提示对象已恢复及失败步骤，不显示整体成功；不要把该提示理解为对象未改变。

### 4.5 附件

`/attach <subcommand>`：
- `list <obj_id>`         —— 列出对象附件
- `add <obj_id> <path>`   —— 添加附件
- `rename <obj_id> <aid> <new>` —— 重命名
- `delete <obj_id> <aid>` —— 软删除
- `restore <obj_id> <aid>` —— 恢复
- `purge <obj_id> <aid>`  —— 永久删除
- `cleanup`               —— 重试当前账户删除意图并清理可证明归属的孤儿附件

`/attach cleanup` 需要解锁，无对象 ID 参数。清理在同一 root 的维护窗口内执行：有附件写入等在途 activity 时拒绝并可稍后重试；明确删除意图仍 pending 时停止。孤儿扫描保护当前/软删除对象、回收站、历史、数据库 Profile、未决同步冲突和导入恢复引用；无法完整认证、归属不明或仍被引用的文件保留。结果显示完整移除目录数、实际移除逻辑字节、保留数和失败数；失败或整轮停止不会显示整体完成。细节与验证状态见 [附件规范 §11](../attachment-storage-spec.md#11-原会话内的孤儿附件清理rf-903)。

### 4.6 备份

`/backup <subcommand>`：
- `list`                 —— 列出当前账户备份
- `create [name]`        —— 创建新备份
- `restore <name>`       —— 恢复（强制 Y/n 确认）
- `delete <name>`        —— 删除备份

CLI 可恢复 GUI 2.0 的 `data_b64` 和旧版 `data` 字节数组备份。两者同时存在时优先非空 `data_b64`，空字符串回退到 `data`；显式空字符串或空数组可表示空数据，缺失两种数据则拒绝恢复。全部 Profile 解码成功后才开始写入，非法 Base64 不会回退为旧数组或空内容，也不会因后续条目解码失败而提前覆盖已有 Profile；空 `profiles` 清单仍可恢复（RF-011）。

GUI 和 CLI 已共用备份编解码规则（RF-013），CLI 创建仍为 2.0/字节数组，GUI 创建仍为 2.0/Base64。恢复接受字符串版本 1.0/2.0 的同结构清单；版本、创建时间、声明数量和条目列表必须存在，声明数量必须正确。未知版本、缺少清单头或数量不符会在写入前报错。恢复保留原 Profile ID 和未出现在清单中的记录，不会把其他 Profile 自动切换为当前主 Profile；旧格式的逐条数据库保存方式保持不变。

当前备份恢复仅保存 Profile，不重建对象和附件实体。`/attach cleanup` 保护数据库内的恢复引用，未扫描 `backups/*.solosoul_backup` 文件；metadata-only 包中旧附件路径不保证因保留包文件而永久可用。

### 4.7 加密导出/导入

| 命令 | 说明 |
|------|------|
| `/export [包路径] --full\|--pages <分类列表>\|--objects <对象ID列表> [--include-attachments]` | 加密 ZIP 导出（`.solosoul`，包含 `manifest.json`、`payload.enc`、可选 `attachments/`；列表使用逗号分隔） |
| `/import <包路径> [--preview] [--strategy skip\|overwrite\|merge]` | 导入或预览 `.solosoul` 包 |
| `/import --pending` | 查看当前账户未完成的导入任务 |
| `/import --resume <任务ID> [同一导入包路径]` | 继续原任务的附件及偏好处理 |

导出密码通过模态提示采集，**不允许与主密码**相同。

默认不导出附件文件；`--include-attachments` 表示导出所选对象中的全部未删除附件，未选对象的附件不会进入包。范围在共享 Core 入口映射为 `AttachmentExportScope::None/All`，与 GUI 手动空数组的零选择契约分别适配（RF-015）。当前 CLI 导出兼容入口未传入 Vault 附件密钥，选择 SOLC 静态密文会明确报错；RF-023已将完整导出用例与原子writer收敛至Core，失败不会覆盖既有备份；LegacyDirect兼容包与无源密钥行为保持。

导入中断后，对象记录可能已提交，附件或偏好仍待处理。重新解锁原账户，使用 `/import --pending` 查看任务，再按原 ID 继续。续接按需索取同一包及包密码，并核对包内容；已保存就绪材料或已完成的任务无需源包和密码。续接不能更改策略或选择。续接时若路径包含空格，可省略路径参数，在提示框输入。已写文件数与可用附件数分别统计；文件发布后，附件元数据仍可能待提交。

### 4.8 设置与安全

| 命令 | 说明 |
|------|------|
| `/language`            | 切换 CLI 语言（写入 `ui_preferences.json`，无参时显示当前） |
| `/theme`               | 切换主题（写入 `ui_preferences.json`，无参时显示当前） |
| `/setting`（无参）     | **打开设置菜单**（等价于首页点击『设置』） |
| `/setting <key> <val>` | 修改加密的账户偏好 |
| `/security`            | 修改主密码 / 提示词 / 回收站保留天数 / 删除账户 |
| `/debug_log`           | 导出诊断包 |

### 4.9 LLM

| 命令 | 说明 |
|------|------|
| `/model`                       | 切换默认 provider/model |
| `/llm_config`                  | 列出当前 LLM 配置 |
| `/llm_stats`                   | 用量统计 |
| `/llm_list_conversations`      | 列出对话历史 |
| `/llm_conversations`           | 同上 |
| `/llm_chat [model_id]`         | 进入 CLI 聊天 REPL（流式响应） |

### 4.10 插件

| 命令 | 说明 |
|------|------|
| `/plugin` 或 `/plugin_list` 或 `/plugin-market` 或 `/plugin_market` | 插件列表（可按名称过滤） |
| `/plugin_run <id> [args...]` | 运行插件 |
| `/plugin_install <id>` | 登录后后台下载、校验并安装 |
| `/plugin_update <id>` | 登录后后台准备注册表最新版本，发布成功后切换 |
| `/plugin_cancel <id>` | 请求取消尚未提交的安装或更新 |
| `/plugin_uninstall <id>` | 卸载 |
| `/plugin_sessions` | 列出插件会话 |
| `/plugin_list_installed` | 已安装列表 |
| `/plugin_audit_log` | 插件审计日志 |
| `/plugin_registry_update` | 更新本地市场 registry |
| `/plugin_search <kw>` | 按关键词搜索 |

插件列表下方显示实际下载字节、安装阶段和取消命令；其他页面的状态栏显示活动安装。插件页可直接输入斜杠命令，下载期间可继续输入、切换页面或锁定。同一插件的活动任务结束前不能再次安装、更新、运行或卸载；取消保留占位直至真正回收，之后可以重试。已有插件运行任务尚未结束时，也不能启动新的安装或更新。

`/plugin_list_installed` 只显示结构完整的安装，运行前另做 WASM 内容校验。后台成功后同时刷新当前页面和返回页面缓存中的列表/详情，不强制跳回插件页。下载、校验或发布失败保留原已安装版本；锁定、同账户重登或切换账户使未提交任务失效。最终提交已经开始时等待真实结果，取消不删除已发布版本，旧会话也不能把完成提示写进新会话。

安装与更新均选择注册表的最新版本；远程获取失败时可采用注册表已知且校验通过的 bundled 版本，完成提示显示实际版本。CLI 沿用全局插件目录 `~/.solosoul/plugins/`，不随账户、`--data-dir` 或 `SOLOSOUL_DATA_DIR` 改变。安装使用完整版本目录与 `current.json` 指针，没有指针时仍可读取旧版 `manifest.json` + `plugin.wasm`；旧客户端可能看不到新安装，或只读到遗留旧版，降级不能作为可靠回退。具体发布和读取约定见[插件运行时](../plugin_market/02-runtime.md)。插件运行和注册表刷新仍使用原入口，本项只迁移显式安装与更新任务。

### 4.11 设备同步  ← *本期新增*

`/sync <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `status` / `list` | 列出当前账户 vault 已持久化的 peers |
| `with <peer-or-host:port>` | 启动后台一次性同步，立即返回输入循环 |
| `jobs` | 查看同步任务 ID 和启动、同步、收尾阶段 |
| `cancel [task-id]` | 请求取消指定任务；省略 ID 取消当前同步 |
| `trust <peer>` | 将 peer 标记为受信任 |
| `untrust <peer>` | 取消 trust |
| `forget <peer>` | 从 vault 中移除 peer |
| `help` | 帮助 |

`/sync with` 使用 [Tasks](../../solosoul_cli/src/tasks.rs) 的协作异步入口，复用 [shared_runtime](../../solosoul_cli/src/util.rs)。[同步命令](../../solosoul_cli/src/commands/sync.rs) 立即返回输入循环，等待期间可输入、重绘与处理 Tick/自动锁定。后台阶段与终态带原 task ID，通过原账户、会话代次和实际任务记录校验接纳；完成不强制切换当前页面。

同一 CLI 会话只接纳一个尚未收尾的同步任务，重复 `/sync with` 提示查看/取消；请求取消后仍保留占位，实际终态接纳后才能重试。`/sync jobs`（或 `/sync status`）显示 ID 与阶段；`/sync cancel [task-id]` 只请求取消，不能据此认为网络/数据库工作已结束。

每次创建独立 SyncManager，原会话内准备 identity，执行 start → sync → stop_and_wait；启动、配对或同步失败及取消均等待真实收尾。取消保留原30秒停机宽限期和系统网络超时，不承诺立即完成。锁定/退出请求取消并清除任务展示，旧会话的完成或进度不能恢复页面；正常退出等待真实 join 后返回。已提交同步记录不因取消回滚，未信任/配对失败仍为失败，不自动信任或报成功。CLI 不维持常驻同步服务，不改协议或 SAS。实现与本机验证见 [RF-213](../REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-213) 和[验证记录](../verification/rf213-cli-sync-task-2026-10-01.json)。

### 4.12 本地 OCR  ← *本期新增*

`/ocr <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `tiers` | 列出 tiny / small / medium 档位及其本地安装状态 |
| `scan <path>` | 在后台识别本地图片或 PDF；PDF 优先读取文本层，否则渲染后逐页识别 |
| `scan --mrz <path>` | 在后台识别护照图片中的 MRZ（证件号/国籍/有效期 + checksum） |
| `jobs` | 显示任务 ID 和排队、运行、请求取消状态 |
| `cancel [task-id]` | 取消指定任务；省略 ID 取消本会话全部 OCR 任务 |
| `result` | 打开本会话最近一次完成的识别结果 |
| `status` | 当前模型目录与已安装档位 |
| `help` | 帮助 |

环境变量 `SOLOSOUL_OCR_TIER=tiny|small|medium` 控制档位（默认 `small`）。
模型目录：`{SOLOSOUL_DATA_DIR}/models/pp-ocr-v6-{tier}/`。
若档位未安装，CLI 会提示通过 GUI 安装或手动放置。

扫描提交后立即返回事件循环，可继续输入、重绘和自动锁定。最多一个任务运行、四个排队；尚未接纳的完成事件仍占用名额。任务页命令框为空时，`Esc` 请求取消全部 OCR；`/back` 只离开任务页。

取消排队任务不会加载模型。运行中的取消要等当前模型加载或不可中断的原生推理返回，再在阶段/页边界停止；“请求取消”不代表已经结束。任务拥有者等待真实退出后才释放运行名额，PDF 临时页也在实际使用结束后回收。退出 CLI 同样等待这些任务。

完成时若仍在任务页则展示结果，否则保留当前页面；`/ocr result` 只缓存最近一份结果。锁定、同账户重新解锁或切换账户会撤销旧任务结果，并清除当前页、返回页和最近结果中的 OCR 正文。MRZ 无识别结果仍显示明确错误，不记为成功。

### 4.13 Embedding 模型  ← *本期新增*

`/embed_model <subcommand> [args]`：

| 子命令 | 说明 |
|--------|------|
| `list` | 列出已安装模型，以及下载字节进度和取消状态 |
| `install <model_id>` | 登录后从注册表后台下载，完成校验后安装 |
| `cancel <model_id>` | 取消尚未进入最终提交的下载，等待临时文件清理 |
| `remove <model_id>` | 删除本地模型目录 |
| `status` | 显示本地目录（CLI 不直接读写 GUI 端 LlmConfig） |
| `help` | 帮助 |

环境变量 `SOLOSOUL_EMBED_REGISTRY=https://...embed-registry.json` 覆盖默认 URL。
本地目录：`{SOLOSOUL_DATA_DIR}/embed_models/<model_id>/model.bin`。
下载期间可继续输入命令、切换页面或锁定；`/embed_model list` 可查看进度。同一模型的下载在实际结束前不能重复启动。取消、下载失败或提交前锁定会清理本次临时文件，随后可重试；最终提交已开始时会等待真实结果，已完成的模型不会被取消命令删除。只有普通、非空模型文件才列为已安装，空目录可直接重新安装；存在的模型文件不会被覆盖。

注册表保持顶层 `models` 数组；每项包含 `id`、`name`、`size_mb`、`sha256`、`download_url`，非空 `sha256` 必须匹配；空摘要沿用既有兼容规则。旧模型没有存储摘要，因此列表显示不代表重新验证过历史文件的完整性。

此命令保持 CLI 的原始二进制格式；GUI 使用另一模型安装目录和 ONNX/tokenizer 格式，CLI 下载不会自动安装或激活 GUI 模型。GUI 模型仍需通过 GUI 管理。

### 4.14 设置菜单  ← *本期新增*

首页『设置』或键入 `/setting`（无参）打开此菜单，包含 4 项；每项支持鼠标点击 + 键盘上下选择。

| 菜单项 | 功能 | 是否需解锁 |
|------|------|----------|
| 语言 | 弹列表（简体中文 / English / 日本語），回车即写 `ui_preferences.json` + 绿色 toast | 否 |
| 主题 | 弹列表（跟随系统 / 浅色 / 深色），同上 | 否 |
| 自定义偏好键值 | 连续 prompt 输入键名 + 值，写入当前账户加密 profile preferences；JSON 尝试解析，否则按字符串 | 是 |
| 导出调试包 | 等价于 `/debug_log`，导出至 `~/.solosoul/logs/` | 是 |

退出请按 Esc。命令行兼容：`/language <code>`、`/theme <主题>`、`/setting <key> <value>`、`/debug_log [文件名]` 仍保留以兼容脚本。

## 5. 进程锁与并发

[CLI 启动](../../solosoul_cli/src/main.rs) 使用 `VaultService::try_with_base_path` 取得 [VaultRootOwner](../../tauri/crates/solosoul-vault/src/root_owner.rs)，成功后才创建日志 writer、加载账户和进入 TUI。桌面 GUI 同样必须取得 owner；同一 canonical Native root 的独立实例竞争失败立即返回错误，不能继续打开数据库、迁移或写入业务数据。锁与旧 [ProcessLock](../../tauri/crates/solosoul-core/src/process_lock.rs) 互操作；同应用连接复用必须显式共享已有 owner。

状态栏、About 和 `/doctor` 查询当前服务已有的 owner，不再次获取锁。`/logout`、`/lock` 和五分钟自动锁定使会话失效；服务、Store、Session、FS 克隆、真正后台 worker 和实际日志 writer 保留 owner 至各自 Drop。只有最后一个 owner 句柄释放，桌面 OS 锁才关闭；关闭会话或取消任务等待不表示锁已释放。

后台任务派发前取得 activity，真实 worker 完成后才释放。改密、主密码解锁升级、删除账户和目录替换需要同一 root 的维护 guard；已有活动时拒绝并稍后重试。同步维护通过 `stop_and_wait` 等待旧句柄退出，不能仅请求 abort 就继续改密。存在 RF-022 journal 的目录仍拒绝迁移；完整 relocation 仍需专门协议；RF-903 旧未标记附件清理见第4.5节和附件规范 §11。

上述 OS 排他锁适用于遵守协议且声明同一 canonical root 的桌面实例。Android/iOS 的文件锁为 no-op，本机准入 gate 不能锁住远端 SAF provider。实际命令、源码 SHA 与本机 Windows 结果以 [RF-905 验证记录](../verification/rf905-root-ownership-2026-10-01.json) 为准；不据此宣称其他平台、可见 GUI 流程或 release 透明 KDF 升级已实测。

## 6. 自动锁定

- 已登录状态下 5 分钟无键盘或鼠标操作自动锁定 Vault，会话密钥 zeroize；
- 模态提示（密码、确认对话框）打开期间暂停自动锁定计时；
- 状态栏显示倒计时（`锁定倒计时: 240s`）。
- 窗口大小变化和后台进度不会重置倒计时；锁定或切换账户后，旧后台消息不再写回界面。

## 7. 日志

CLI 成功取得 root owner 后才写入 `{DATA_DIR}/logs/cli.log`，**不输出主密码或 session key**。实际日志 writer 保留同一 owner，直到真正线程退出；不能把 `WorkerGuard` 的等待返回当作日志线程已退出。
`/doctor` 中列出日志路径，便于排错。

## 8. 常见问题

| 问题 | 解决方案 |
|------|----------|
| "无本地账户" | 第一次启动请在 Welcome 页按 Enter 走创建账户向导 |
| "Vault 未解锁" | 先执行 `/unlock` 或在登录向导中输入密码 |
| `/ocr scan` 报"模型未安装" | 通过 GUI 安装或放置模型到 `models/pp-ocr-v6-{tier}/` |
| `/embed_model install` 报"注册表 schema 不匹配" | 假定 schema `{"models":[{id,name,size_mb,sha256,download_url}]}`；现网注册表协议可能不同，见 §4.13 风险说明 |
| `/sync with` 仍在等待 | 输入循环可继续工作；`/sync jobs` 查看阶段、`/sync cancel` 请求停止。收尾等待真实退出，取消不回滚已提交记录，见 §4.11 |
| 如何释放 CLI 进程锁 | 退出 CLI 并等待实际 worker、日志 writer 和所有 owner 句柄释放；自动锁定和 `/logout` 不释放目录所有权，见 §5 |

## 9. 命令兼容性

GUI 与 CLI 复用部分核心 crate 和数据格式，入口参数、错误文本、存储细节及任务生命周期并非逐项一致。当前可确认的共享行为如下。

| 领域 | 当前边界 |
|------|----------|
| 对象回滚 | 已复用共享用例，见 [RF-008](../REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-008) |
| 对象创建 | 已共享模板初始化规则，见 [RF-010](../REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-010) |
| Profile 备份 | 已共享兼容解码和清单，见 [RF-013](../REFACTOR_EXECUTION_REPORT_2026-09-25.md#rf-013)；GUI 写 Base64，CLI 写字节数组，不能称为完全相同的编码 |
| `.solosoul` 导出/导入 | RF-023后完整导出共用Core及原子writer，GUI/云/恢复的Advanced与旧CLI的LegacyDirect包字段差异保持；CLI选择SOLC源仍明确缺密钥错误。RF-024 后完整导入与 GUI/云/恢复共用 Core 提交和恢复执行；CLI 的默认策略、重复写计数及旧历史保持继续兼容。实际验证见 [RF-024 证据](../verification/rf024-core-encrypted-import-2026-10-01.json) |
| 设置与进程锁 | CLI `/language` / `/theme` 写 UI 偏好文件；GUI 解锁后另有账户加密偏好优先级。桌面同 root 目录所有权与移动 no-op 边界见 §5 |
| 同步、OCR、Embedding | GUI 已有设备同步、OCR 页面及本地模型面板；CLI 的后台一次性同步/取消/退出等待见 §4.11，Embedding 格式与安装目录差异见 §4.13 |

GUI 页面入口可从 [routes.tsx](../../tauri/src/App/routes.tsx) 和 [LlmConfigPage.tsx](../../tauri/src/pages/ai/LlmConfigPage.tsx) 核对。后续按领域分别收敛，不将所有命令的路径、参数、错误字符串写成“1:1 对齐”。

## 10. 发布与下载

SoloSoul CLI 二进制随 `vX.Y.Z` Tag 自动发布到 GitHub Releases：
- `artifacts/cli/macos-aarch64/solosoul` — macOS arm64
- `artifacts/cli/macos-x86_64/solosoul` — macOS x86_64
- `artifacts/cli/windows/solosoul.exe` — Windows x86_64

⚠️ **macOS CLI 当前未公证**（未签约 / 未 notarize）。首次启动可能被 Gatekeeper 隔离。应急跳过命令：

```bash
which solosoul                                           # 先确认实际安装路径 (Homebrew/源码可能不在 /usr/local/bin)
xattr -dr com.apple.quarantine "$(which solosoul)"        # 只对单个文件，避免误伤同目录其它二进制
```

或右键 → 打开 → 确认。正式分发需 codesign + notarize,留待后续 PR。

⚠️ **Windows CLI 同样未签名**。未签名的 PE 在 Win10/11 上会被 SmartScreen 拦截（“Windows protected your PC”），点 “More info → Run anyway” 可跳过。正式分发需 EV 代码签名证书或加入微软 ISV 认证。

RF-024 将完整导入迁入 Core。CLI 的 `/import` 与 `resume` 仍按原流程采集口令、捕获原会话、显示操作 ID 和部分进度；策略为 `skip` / `overwrite` / `merge`，后两者仍沿用原覆盖含义。CLI 保留每次对象写操作计数与本地旧历史，不因共享服务新增 GUI 的 KeepBoth 或导入包历史行为。对象批次与任务接纳、附件/偏好恢复共用 Core 管线，同 ID 重试和 Complete 读取不重复写计数。
