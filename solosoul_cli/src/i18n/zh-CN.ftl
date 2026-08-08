## SoloSoul CLI — 中文翻译

### App / Global
app-title = SoloSoul CLI
app-tagline = 独奏生命数据，重塑数字原点
app-tagline-short = 本地优先 · 零知识 · 你的数据你做主
welcome-back = SoloSoul · 欢迎回来，{$name}
welcome-back-full = SoloSoul · 欢迎回来，{$name} · {$id}

### Navigation hints
hint-up-down-enter = ↑/↓ 选择 · Enter 确认
hint-up-down-enter-esc = ↑/↓ 选择 · Enter 确认 · Esc 返回
hint-enter-esc = Enter 确认 · Esc 返回
hint-enter-esc-quit = Enter 下一步 · Esc 退出
hint-esc-back = Esc 返回
hint-esc-or-back = 按 Esc 或输入 /back 返回
hint-click = ↑/↓ 选择 · Enter 确认 · Esc 返回 · 鼠标可点击

### Locked / Welcome
locked-title = 已锁定
welcome-title = 欢迎
welcome-desc = 未发现本地账户。请先使用 GUI 客户端创建账户，或启动创建账户向导。

### Unlock
unlock-select-account = 选择账户登录
unlock-enter-password = 输入主密码
unlock-account-info = 账户：{$name} · {$id} · 密码提示词：{$hint}
unlock-password-warning = 主密码不会被保存，无法找回。
unlock-biometric-hint = · B 使用 {$type}
unlock-hint-account-list = 使用 ↑/↓ 选择，Enter 确认，Esc 取消

### Onboarding
onboarding-create-account = 创建账户
onboarding-enter-name = 请输入账户名（用于本地标识，可自定义）
onboarding-enter-password = 设置主密码
onboarding-password-length = 主密码至少需要 8 位。
onboarding-confirm-password = 确认主密码
onboarding-confirm-desc = 请再次输入主密码以确认无误。
onboarding-enter-hint = 密码提示词（可选）
onboarding-hint-desc = 当忘记主密码时，提示词可帮助你回忆。可直接留空。
onboarding-confirm-title = 确认创建账户
onboarding-confirm-name = 账户名：{$name}
onboarding-confirm-pw-masked = 主密码：******
onboarding-confirm-hint = 提示：{$hint}
onboarding-confirm-hint-none = （无）
onboarding-confirm-import-desc = 创建后将导入默认模板，并直接进入首页。
onboarding-exit-prompt = 退出创建账户？未保存的数据将不会被保留。

### Home
home-shortcut-list = 浏览
home-shortcut-list-desc = 列出页面与对象
home-shortcut-search = 搜索
home-shortcut-search-desc = 全局关键词搜索
home-shortcut-create = 创建
home-shortcut-create-desc = 新建对象
home-shortcut-trash = 回收站
home-shortcut-trash-desc = 查看已删除项目
home-shortcut-settings = 设置
home-shortcut-settings-desc = 账户偏好设置
home-shortcut-help = 帮助
home-shortcut-help-desc = 查看全部命令
home-shortcut-plugins = 插件
home-shortcut-plugins-desc = 浏览插件市场
home-hint = ↑/↓ 选择，Enter 填入命令，直接输入 /help 查看全部命令

### Object list
object-list-title = 页面列表
object-list-empty = 暂无内容
object-list-truncated = · 结果已截断至前 200 条
object-list-table-id = ID
object-list-table-name = 名称
object-list-table-type = 类型
object-list-table-sensitivity = 敏感度

### Object detail
object-detail-name = 名称：{$name}
object-detail-id = ID：{$id}
object-detail-type = 类型：{$type}
object-detail-section = 分区：{$section}
object-detail-sensitivity = 敏感度：{$level}
object-detail-version = 版本：{$ver}
object-detail-sensitive-masked = 敏感对象：属性值已掩码。编辑模式下可验证主密码后查看。

### Size / Stats
size-title = 账户统计
size-pages = 页面数量：{$count}
size-objects = 对象数量：{$count}
size-trash = 回收站项目：{$count}
size-profiles = Profile 数量：{$count}
size-total-size = 总大小：{$size}

### Search
search-title = 搜索「{$query}」· 共扫描 {$count} 项
search-no-results = 未找到匹配结果。

### History
history-title = 对象 {$id} 的历史快照
history-empty = 暂无历史快照。

### Trash
trash-title = 回收站
trash-empty = 回收站为空。
trash-hint = ↑↓ 移动 · 空格 多选 · R 恢复 · P 彻底删除 · Esc 返回

### Backup
backup-title = 备份列表
backup-empty = 暂无备份。
backup-hint = 使用 /backup restore <id> 恢复，/backup delete <id> 删除。
backup-created = 备份「{$name}」已创建（{$size}）
backup-deleted = 备份已删除
backup-restore-success = 已恢复：{$id}

### Operation log
log-title = 审计日志 · 共 {$count} 条
log-empty = 暂无审计日志。
log-export-hint = 使用 /export_log [文件名] 导出完整日志。

### Doctor
doctor-title = 诊断报告
doctor-data-dir = 数据目录：{$path}
doctor-lock-status = 进程锁：{$status}
doctor-lock-acquired = 已持有（GUI 不可用）
doctor-lock-none = 未独占
doctor-account-issues = 账户异常：
doctor-accounts = 账户数量：{$count}
doctor-account-count = 账户数量：{$count}
doctor-core-version = 核心库版本：{$ver}
doctor-vault-version = Vault 版本：{$ver}
doctor-platform = 平台：{$os} / {$arch}
doctor-log-path = 日志路径：{$path}

### Settings menu
settings-title = 设置
settings-language = 语言
settings-language-desc = 切换界面语言
settings-theme = 主题
settings-theme-desc = 切换界面主题（跟随系统 / 浅色 / 深色）
settings-preference = 自定义偏好
settings-preference-desc = 写入加密 profile preferences 中的任意键值对
settings-debug-log = 导出调试包
settings-debug-log-desc = 导出审计日志 + 脱敏系统信息到 logs/

### Settings select
settings-current = 当前
current-language = 语言已设置为：{$code}
current-theme = 主题已设置为：{$code}

### Profile
profile-title = Profile
profile-id = ID：{$id}
profile-name = 名称：{$name}
profile-version = 版本：{$ver}
profile-updated = 更新时间：{$time}

### Template
template-title = 模板库
template-empty = 暂无模板。
template-detail-title = 模板详情
template-hint = ↑↓ 移动 · Enter 查看详情 · D 删除用户模板 · Esc 返回
template-field-count = 字段数
doctor-source = 来源：{$source}
profile-hint = Esc 返回 · /profile set <路径> <值> 编辑字段

### Attachment
attachment-list-title = 附件列表 - {$id}
attachment-list-title-deleted = 附件列表（含已删除）- {$id}
attachment-empty = 暂无附件。
attachment-hint = 使用 /attach add <path> 添加，/attach delete <id> 删除，/attach purge <id> 彻底删除。
attachment-added = 已添加附件：{$path}
attachment-renamed = 已重命名为：{$name}
attachment-deleted = 已删除附件：{$id}
attachment-restored = 已恢复附件：{$id}
attachment-purged = 已彻底删除附件：{$id}
attachment-soft-delete-prompt = 软删除附件「{$id}」？可在回收站恢复。
attachment-purge-prompt = 彻底删除附件「{$id}」？此操作不可恢复。

### Sync
sync-title = 同步状态
sync-peers-from-vault = vault 中已持久化的 peer（来自历史同步会话；不包含当前 mDNS 实时发现）
sync-no-peers = 暂无 peer。
sync-unknown-peer = 未知 peer
sync-with-success = /sync with {$peer} 完成：{$summary}。详细计数在审计日志中。
sync-with-failure = /sync with {$peer} 失败：{$err}
sync-trusted = 已将 peer {$id} 标记为 trusted
sync-untrusted = 已将 peer {$id} 标记为 untrusted
sync-forgotten = 已从 vault 中删除 peer {$id}
sync-need-unlock = Vault 未解锁，请先 /unlock
sync-no-account = 无当前账户

### OCR
ocr-result-title = OCR · {$source}
ocr-no-text = （未识别到文本）

### Help
help-title = 帮助
help-topic = 命令：{$cmd}
help-header = 使用 /help <命令> 查看具体用法
help-unknown-topic = 未知命令，使用 /help 查看全部命令。
biometric-generic-name = 生物识别

### About
about-title = 关于
about-version = 版本：{$ver}
about-platform = 平台：{$os} / {$arch}
about-data-dir = 数据目录：{$path}
about-lock = 进程锁：{$status}
about-lock-acquired = 已持有（GUI 不可用）
about-lock-none = 未独占

### LLM
llm-config-title = LLM 配置
llm-active-provider = 当前活跃：{$name}
llm-chat-ai-prefix = AI：
llm-chat-error-prefix = 错误：
llm-conversation-empty = 暂无对话记录
llm-stats-title = LLM 使用统计

### Plugin
plugin-list-title = 插件列表（共 {$count} 个）
plugin-detail-title = 插件详情：{$name}

### Embed Model
embed-model-title = Embedding 模型列表
embed-model-install-hint = 用 `/embed_model install <id>` 从注册表下载并安装。
embed-model-registry-hint = 注册表 URL 由环境变量

### External editor
ext-editor-date = {$label} · 日期
ext-editor-datetime = {$label} · 日期时间
ext-editor-time = {$label} · 时间（HH:MM:SS）
ext-editor-select-action = {$label} · 选择操作
ext-editor-edit-label = [编辑] {$i}. {$label}
ext-editor-delete-label = [删除] {$i}. {$label}

### Status bar
status-locked = 🔒 已锁定
status-unlocked = 🔓 已解锁 · {$id}
status-unlocked-with-name = 🔓 已解锁 · {$name} · {$id}
status-object = 对象：{$name}
status-lock-countdown = 锁定倒计时：{$sec}s
status-settings = 设置 · 语={$lang} · 主={$theme}

### Generic messages
generic-yes = 是
generic-no = 否
generic-none = 无
loading = 加载中...
unknown-command = 未知命令：{$cmd}
exit-prompt = 退出 SoloSoul CLI？
session-timeout = 会话已超时锁定。
no-results = 无匹配结果。

### Command messages
cmd-no-previous-screen = 没有上一屏可返回
cmd-no-accounts = 未发现本地账户
cmd-already-logged-in = 当前已登录
cmd-no-accounts-gui = 未发现本地账户，请使用 GUI 客户端创建账户
cmd-not-logged-in = 当前未登录
cmd-need-unlock = 请先使用 /unlock 登录
cmd-vault-not-open = Vault 未打开
cmd-vault-locked = Vault 未解锁
cmd-need-login = 未登录
cmd-no-pages = 暂无页面，请先使用 /newpage 创建页面
cmd-no-current-account = 没有当前账户
cmd-not-in-wizard = 当前不在向导中
cmd-no-changes = 当前没有可保存的更改
cmd-page-not-found = 页面「{$name}」不存在
cmd-object-not-found = 对象「{$id}」不存在或已被删除
cmd-provide-object-id = 请提供对象 ID，例如 {$cmd} obj_xxx
cmd-provide-page-name = 请提供页面名称，例如 /newpage 旅行
cmd-provide-trash-id = 请提供 trash_id，例如 /restore trash_xxx
cmd-profile-serialize-failed = 序列化 profile 数据失败：{$err}
cmd-unknown-subcommand = 未知子命令：{$cmd}
cmd-operation-failed = 操作失败：{$err}
cmd-saved = 已保存
cmd-deleted = 已删除：{$name}
cmd-restored = 已恢复：{$id}
cmd-prompt-delete-page = 页面「{$name}」下包含 {$count} 个子对象，删除将一并移入回收站，确认？
cmd-page-deleted = 页面「{$name}」及 {$count} 个子对象已删除
cmd-backup-created = 备份已创建：{$id}（{$size} 个 Profile，{$bytes} 字节）
cmd-backup-restored = 备份「{$id}」已恢复
cmd-backup-deleted = 备份「{$id}」已删除
cmd-backup-name-empty = 备份名称不能为空
cmd-trash-item-not-found = 回收站项目「{$id}」不存在
cmd-purge-prompt = 彻底删除「{$name}」？此操作不可恢复。
cmd-batch-restore-result = 恢复完成：成功 {$success} 项；失败 {$failed} 项
cmd-restored-cascaded-page = 已连同所属页面 "{$page}" 一并恢复
cmd-restored-cascaded-count = 已恢复页面及其中 {$count} 个对象
cmd-restored-rebuilt-page = 所属页面已被永久删除，已重建页面 "{$page}" 并恢复对象
cmd-batch-purge-result = 彻底删除完成：成功 {$success} 项；失败 {$failed} 项
cmd-provide-object-id-or-detail = 请提供对象 ID 或在对象详情页执行
cmd-execute-in-detail = 请在对象详情页执行 {$cmd}
cmd-provide-attachment-id = 请提供附件 ID，例如 {$cmd} att_xxx
cmd-provide-filename = 请提供新文件名
cmd-profile-rename-usage = 用法：/profile rename <名称>
cmd-profile-set-usage = 用法：/profile set <路径> <值>
cmd-profile-updated = Profile 名称已更新为：{$name}
cmd-preference-updated = 偏好已更新：{$key}
cmd-setting-usage = 用法：/setting <key> <value>
cmd-provide-backup-id = 请提供备份 ID，例如 /backup {$cmd} weekly_20260101_120000
cmd-backup-usage = 用法：/backup list | create <name> | restore <id> | delete <id>
cmd-export-import-usage = 用法：/export [选项] | /import <路径> [选项]
cmd-provide-import-path = 请提供要导入的文件路径
cmd-import-success = 成功导入 {$count} 个对象
cmd-password-min-length = 主密码至少需要 8 位
cmd-password-mismatch = 两次输入的新密码不一致
cmd-password-changed = 主密码已修改
cmd-password-hint-updated = 密码提示已更新为：{$text}
cmd-password-hint-current = 当前密码提示：{$hint}
cmd-trash-retention-usage = 用法：/security trash-retention <天数>
cmd-trash-retention-set = 回收站保留天数已设置为：{$days} 天
cmd-biometric-not-supported = 当前平台不支持生物识别
cmd-biometric-enabled = 生物识别登录已启用
cmd-biometric-disabled = 生物识别登录已关闭
cmd-biometric-test-passed = 生物识别测试通过
cmd-biometric-test-unavailable = 生物识别测试不可用
cmd-account-deleted = 账户已删除
cmd-password-wrong-canceled = 密码错误，账户删除已取消
cmd-verify-failed = 验证失败：{$err}
cmd-template-show-usage = 用法：/template show <id>
cmd-template-delete-usage = 用法：/template delete <id>
cmd-template-deleted = 已删除用户模板：{$id}
cmd-load-failed = 加载失败：{$err}
cmd-export-import-unknown = 未知的导出/导入子命令：{$cmd}
cmd-key-empty = 键名不能为空

### Security commands
cmd-security-usage = 用法：/security password|hint|trash-retention|delete-account|biometric
cmd-biometric-status = 生物识别：{$status} · {$configured} · 类型：{$kind} · {$error}
cmd-biometric-usage = 用法：/security biometric status|enable|disable|test
prompt-current-password = 当前主密码
prompt-new-password = 新主密码
prompt-confirm-password = 确认新主密码
prompt-enable-biometric = 输入当前主密码以启用生物识别
prompt-disable-biometric = 输入当前主密码以关闭生物识别
prompt-delete-account = 输入当前主密码以确认删除账户
prompt-delete-account-confirm = ! 删除账户将永久清除所有数据，是否继续？
prompt-export-password = 导出密码
prompt-import-password = 导入密码

### Export/Import
cmd-export-success = 已导出 {$count} 个对象到 {$path}
cmd-import-preview = 导出包预览：版本 {$version}，{$count} 个对象，包含附件：{$has}，密码提示：{$hint}
cmd-export-password-too-short = 导出密码至少需要 8 位
cmd-export-password-format = 导出密码必须同时包含字母和数字
cmd-export-password-same-master = 导出密码不能与主密码相同
cmd-export-password-verify-failed = 校验主密码失败：{$err}
cmd-export-too-many-args = 多余的文件参数
cmd-export-no-scope = 请指定 --full、--pages 或 --objects 之一
cmd-import-need-strategy = --strategy 后需要策略值（skip/overwrite/merge）
cmd-account-not-found = 未找到当前账户

### LLM commands
cmd-llm-need-login = 未登录，无法查看 LLM 模型配置
cmd-llm-vault-locked = Vault 未解锁
cmd-llm-no-active-provider = 未设置活跃 LLM 提供商。使用 /llm_config 配置。
cmd-llm-config-failed = 加载 LLM 配置失败：{$err}
cmd-llm-stats-failed = 加载 LLM 统计失败：{$err}
cmd-llm-list-failed = 加载对话列表失败：{$err}
cmd-llm-need-login-chat = 未登录，无法使用 LLM 聊天
cmd-llm-loaded-conversation = 已加载对话：{$name}

### Sync commands
cmd-sync-usage = 用法：/sync <subcommand> [args]
cmd-sync-trust-usage = 用法：/sync trust <peer>
cmd-sync-untrust-usage = 用法：/sync untrust <peer>
cmd-sync-forget-usage = 用法：/sync forget <peer>
cmd-sync-with-usage = 用法：/sync with <peer-or-host:port>
cmd-sync-runtime-failed = 创建异步运行时失败：{$err}
cmd-sync-trust-operation-failed = /sync trust 操作失败：{$err}
cmd-sync-forget-operation-failed = /sync forget 操作失败：{$err}
cmd-sync-info = vault 中已持久化的 peer（来自历史同步会话；不包含当前 mDNS 实时发现）

### OCR commands
cmd-ocr-usage = 用法：/ocr scan [--mrz] <image-path>
cmd-ocr-unknown-flag = /ocr scan：未知 flag {$flag}。用法：/ocr scan [--mrz] <image-path>
cmd-ocr-extra-arg = /ocr scan：拒绝多余参数 {$arg}。用法：/ocr scan [--mrz] <image-path>
cmd-ocr-image-not-found = 图片不存在：{$path}
cmd-ocr-env-parse-failed = SOLOSOUL_OCR_TIER 解析失败：{$err}
cmd-ocr-tier-not-installed = {$tier} 档位模型未安装。请先通过 GUI 安装或手动放置到 {$path}。
cmd-ocr-engine-failed = 加载 OCR engine 失败：{$err}
cmd-ocr-mrz-not-found = 未在图片中识别到 MRZ 区域
cmd-ocr-mrz-failed = MRZ 识别失败：{$err}
cmd-ocr-scan-failed = OCR 扫描失败：{$err}

### Embed Model commands
cmd-embed-usage = 用法：/embed_model install <model_id>
cmd-embed-remove-usage = 用法：/embed_model remove <model_id>
cmd-embed-already-installed = 模型 {$model} 已经安装
cmd-embed-not-installed = 模型 {$model} 未安装
cmd-embed-installed = 已安装 embedding 模型 {$model}。运行 /embed_model list 查看。
cmd-embed-install-failed = /embed_model install 失败：{$err}
cmd-embed-removed = 已删除 embedding 模型 {$model}
cmd-embed-remove-failed = /embed_model remove 失败：{$err}
cmd-embed-runtime-failed = 创建异步运行时失败：{$err}

### Plugin commands
cmd-plugin-usage-run = 用法：/plugin_run <plugin_id>
cmd-plugin-usage-install = 用法：/plugin_install <plugin_id>
cmd-plugin-usage-update = 用法：/plugin_update <plugin_id>
cmd-plugin-usage-uninstall = 用法：/plugin_uninstall <plugin_id>
cmd-plugin-usage-search = 用法：/plugin_search <keyword>
cmd-plugin-invalid-id = 无效的插件 ID：{$id}（仅允许字母、数字、_ - . 字符）
cmd-plugin-market-empty = 插件市场中暂无可用插件
cmd-plugin-need-login = 未登录，无法运行插件
cmd-plugin-vault-locked = Vault 未解锁
cmd-plugin-not-found = 未找到插件：{$id}
cmd-plugin-running = 正在后台运行插件：{$id} ...
cmd-plugin-run-result = 插件 {$id} 运行完成：exit_code={$code}，fuel={$fuel}
cmd-plugin-run-failed = 插件 {$id} 运行失败：{$err}
cmd-plugin-installed = 插件 {$id} v{$ver} 安装成功
cmd-plugin-install-failed = 安装插件 {$id} 失败：{$err}
cmd-plugin-updated = 插件 {$id} 已更新至 v{$ver}
cmd-plugin-update-failed = 更新插件 {$id} 失败：{$err}
cmd-plugin-uninstalled = 插件 {$id} 已卸载
cmd-plugin-uninstall-failed = 卸载插件 {$id} 失败：{$err}
cmd-plugin-no-sessions = 当前没有活跃的插件会话
cmd-plugin-sessions-header = 活跃会话（共 {$count} 个）：
cmd-plugin-list-sessions-failed = 获取会话列表失败：{$err}
cmd-plugin-none-installed = 本地暂无已安装的插件
cmd-plugin-installed-header = 已安装插件（共 {$count} 个）：
cmd-plugin-list-installed-failed = 获取已安装列表失败：{$err}
cmd-plugin-limit-must-be-positive = limit 必须为正整数
cmd-plugin-limit-must-be-number = limit 必须为数字
cmd-plugin-no-audit-logs = 暂无插件审计日志
cmd-plugin-audit-header = 审计日志（最近 {$count} 条）：
cmd-plugin-audit-failed = 获取审计日志失败：{$err}
cmd-plugin-updating-registry = 正在更新插件注册表...
cmd-plugin-registry-updated = 插件注册表已更新
cmd-plugin-registry-update-failed = 更新注册表失败：{$err}
cmd-plugin-search-no-match = 未找到匹配「{$keyword}」的插件
cmd-plugin-search-failed = 搜索插件失败：{$err}
cmd-plugin-init-failed = 初始化插件管理器失败：{$err}

### History
cmd-history-usage = 请提供对象 ID，例如 /history obj_xxx
cmd-rollback-usage = 请提供对象 ID 与快照 ID，例如 /rollback obj_xxx snap_xxx
cmd-rollback-confirm = 确认将对象「{$obj}」回滚到快照「{$snap}」？当前未保存的更改将丢失。
cmd-rollback-complete = 对象「{$name}」已回滚到快照「{$snap}」

### Search
cmd-search-need-keyword = 请提供搜索关键词，例如 /search 护照

### Template
cmd-template-usage = 用法：/template | /template show <id> | /template delete <id>
cmd-template-source-user = 用户
cmd-template-source-system = 系统
cmd-template-delete-failed = 删除失败：{$err}

### Profile
cmd-profile-usage = 用法：/profile | /profile rename <名称> | /profile set <路径> <值>

### Log
cmd-log-serialize-failed = 序列化日志失败：{$err}
cmd-log-dir-failed = 创建日志目录失败：{$err}
cmd-log-write-failed = 写入导出文件失败：{$err}
cmd-log-exported = 审计日志已导出至：{$path}
cmd-log-vault-not-open = Vault 未打开

### Doctor
doctor-dir-status-label = 数据目录状态：
doctor-dir-writable-label = 数据目录可写：
doctor-lock-status-label = 进程锁状态：
cmd-doctor-no-issues = 无异常。

### New Object
newobj-create-object = 创建对象
newobj-select-page = 创建对象：选择页面
newobj-select-page-desc = 选择一个页面作为新对象的父级，或按 q 取消。
newobj-no-pages = 暂无页面，请先使用 /newpage 创建页面。
newobj-hint-up-down-enter-q = ↑/↓ 选择 · Enter 确认 · q 取消
newobj-blank-object = 空白对象
newobj-select-template = 创建对象：选择模板（页面：
newobj-select-template-desc = 选择模板开始填写字段，或选择「空白对象」仅输入名称。
newobj-fields = 字段
newobj-fill-fields-hint = 按 Enter 编辑字段，s 保存，q 取消。
newobj-fields-nav = ↑/↓ 选择字段 · Enter 编辑字段 · n 修改名称 · s 保存 · q 取消

### Edit Object
editobj-object-info = 对象信息
editobj-properties = 属性
editobj-template = 模板：{$tpl}


### Settings select
settings-lang-title = 选择语言
settings-theme-title = 选择主题
settings-select-hint = ↑/↓ 选择 · Enter 或点击应用 · Esc 取消

### Sync
sync-peers-title = Peer 列表
sync-no-peers-hint = 提示：使用 `/sync with <host:port>` 与 GUI 实例同步后，此处会出现 peer 记录。
sync-subcommand-prefix = 子命令
sync-peers-hint-p1 = 要与某 peer 同步，请先
sync-peers-hint-p2 = 再通过 GUI 启用持续同步。

### OCR
ocr-tiers = 模型档位
ocr-recognized-text = 识别文本
ocr-hint-prefix = 提示
ocr-hint-text = 用 `SOLOSOUL_OCR_TIER=small|medium|tiny /ocr scan <path>` 切换档位；若某档位模型未安装，会提示通过 GUI 或手动放置。
ocr-mrz-hint-text = MRZ 字段来自图像底部机读区；checksum 校验由后端 MRZ 解析器执行。
ocr-mrz-fields = MRZ 字段

### Embed Model
embed-model-not-installed = （本地尚未安装 embedding 模型）
embed-model-list-title = 模型列表

### LLM
llm-active-label = 当前活跃：
llm-providers = 提供商
llm-stats-summary = 总请求：{$count}  |  总 tokens：{$tokens}  |  Prompt：{$prompt}  |  Completion：{$completion}
llm-stats-model = 模型
llm-stats-provider = 提供商
llm-stats-count = 次数
llm-stats-tokens = Tokens
llm-stats-by-model = 按模型
llm-chat-welcome = LLM 聊天已就绪。输入您的问题，按 Enter 发送。输入 /back 返回。
llm-chat-title = LLM 对话
llm-chat-user-prefix = 你：
llm-chat-thinking = AI：⏳ 思考中...
llm-chat-waiting = ⏳ 等待响应中...
llm-chat-input-hint = 输入消息，Enter 发送，Esc 返回
llm-conversation-title = LLM 对话历史
llm-conversation-count-prefix = 共
llm-conversations-label = 对话
hint-up-down-esc-q = ↑↓ 选择  Esc/q 返回
hint-esc-q = Esc/q 返回
hint-up-down-enter-esc-q = 条对话  |  ↑↓ 选择  Enter 打开  Esc/q 返回

### Plugin List
plugin-list-empty = 暂无可用插件
plugin-list-no-match = 无匹配结果
plugin-list-hint = ↑↓ 导航  键入过滤  |  Enter 详情  r 运行  i 安装  u 更新  d 卸载  |  q/Esc 返回
plugin-list-hint-filtering = 输入关键字过滤  Esc 清除  Backspace 删除  |  ↑↓ 导航  Enter 详情  r 运行

### Plugin Detail
plugin-detail-prefix = 插件详情：
plugin-detail-author = 作者：
plugin-detail-category = 类别：
plugin-detail-tier = 等级：
plugin-detail-confirm = 需要确认：
plugin-detail-homepage = 主页：
plugin-detail-core = 要求 Core：
plugin-detail-permissions = 权限：
plugin-detail-no-permissions = 无特殊权限
plugin-detail-network = 网络策略：
plugin-detail-params = 参数：
plugin-detail-required = [必填]
plugin-detail-optional = [可选]
plugin-detail-wasm-hash = WASM SHA256：
plugin-detail-ttl = 数据 TTL：


### Status bar (additional)
status-welcome = 未登录 · 无账户
status-account-list = 账户列表
status-unlock = 登录
status-size = 账户统计
status-doctor = Doctor
status-new-object = 创建对象向导
status-edit-object = 编辑对象向导
status-trash = 回收站
status-onboarding = 创建账户
status-search = 搜索结果
status-history = 历史快照
status-operation-log = 审计日志
status-about = 关于
status-help = 帮助
status-attachment = 附件列表
status-backup = 备份列表
status-profile = Profile
status-template-list = 模板列表
status-template-detail = 模板详情
status-llm-config = LLM 配置
status-llm-stats = LLM 统计
status-conversation = 对话历史
status-llm-chat = LLM 聊天
status-plugin-list = 插件列表
status-plugin-detail = 插件详情
status-sync = 设备同步
status-ocr = OCR
status-embed = Embedding 模型
status-settings-language = 设置 · 语言选择
status-settings-theme = 设置 · 主题选择
status-settings-preference = 设置 · 自定义偏好
status-quit = 退出中
status-lock-held = [L] 进程锁已持有 · GUI 不可用
status-lock-not-exclusive = [!] 未独占


### Commands Phase 3
cmd-attachment-usage = 用法: /attach list [object_id] | add <file_path> | rename <id> <new_name> | delete <id> | restore <id> | purge <id> | cleanup
cmd-provide-file-path = 请提供文件路径，例如 /attach add /path/to/file.pdf
cmd-provide-attachment-id-example = 请提供附件 ID，例如 /attach rename att_xxx new.pdf
cmd-prompt-soft-delete-attachment = 软删除附件 '{$id}'？可在回收站恢复。
cmd-prompt-purge-attachment = 彻底删除附件 '{$id}'？此操作不可恢复。
cmd-cleanup-result = 清理完成：移除 {$count} 个孤立附件，释放 {$bytes} 字节
cmd-prompt-restore-backup =
 确认恢复备份 '{$id}'？
 创建时间: {$date}
 包含 {$count} 个 Profile。
 当前 Vault 中的同名 Profile 将被覆盖。
cmd-prompt-delete-backup =
 确认删除备份 '{$id}'？
 此操作不可恢复。
cmd-llm-current-model =
 当前模型: {$name} - {$model}
 提供商: {$url}
 API 类型: {$api_type}
cmd-template-load-failed = 加载系统模板失败: {$err}
cmd-language-set = 语言已设置为: {$code}
cmd-theme-set = 主题已设置为: {$name}
cmd-preference-value-label = 偏好值（键={$key}，JSON 会被尝试解析，否则按字符串保存）
cmd-debug-log-exported = 诊断包已导出至: {$path}
cmd-ocr-no-models =
 模型目录: {$path}
 未安装任何档位。请从 GUI 安装或下载到该目录。
cmd-ocr-models-status =
 模型目录: {$path}
 已安装: {$installed}
cmd-ocr-status-title = OCR Status（{$path} 目录）
cmd-extra-file-arg = 多余的文件参数
cmd-export-need-scope = 请指定 --full、--pages 或 --objects 之一
cmd-import-need-strategy-value = --strategy 后需要策略值
cmd-export-password-complexity = 导出密码必须同时包含字母和数字
cmd-account-not-found-generic = 未找到当前账户
cmd-export-password-same-as-master = 导出密码不能与主密码相同
cmd-verify-master-failed = 校验主密码失败: {$err}
