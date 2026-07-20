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
loading = 加载中...
unknown-command = 未知命令：{$cmd}
exit-prompt = 退出 SoloSoul CLI？
session-timeout = 会话已超时锁定。
no-results = 无匹配结果。
