## SoloSoul CLI — English translations

### App / Global
app-title = SoloSoul CLI
app-tagline = Solo your life data, reshape your digital origin
app-tagline-short = Local-first · Zero-knowledge · Your data, your rules
welcome-back = SoloSoul · Welcome back, {$name}
welcome-back-full = SoloSoul · Welcome back, {$name} · {$id}

### Navigation hints
hint-up-down-enter = ↑/↓ Select · Enter Confirm
hint-up-down-enter-esc = ↑/↓ Select · Enter Confirm · Esc Back
hint-enter-esc = Enter Confirm · Esc Back
hint-enter-esc-quit = Enter Next · Esc Quit
hint-esc-back = Esc Back
hint-esc-or-back = Press Esc or enter /back to return
hint-click = ↑/↓ Select · Enter Confirm · Esc Back · Mouse clickable

### Locked / Welcome
locked-title = Locked
welcome-title = Welcome
welcome-desc = No local account found. Use the GUI client to create one, or start the onboarding wizard.

### Unlock
unlock-select-account = Select Account
unlock-enter-password = Enter Master Password
unlock-account-info = Account: {$name} · {$id} · Hint: {$hint}
unlock-password-warning = Master password is not stored and cannot be recovered.
unlock-biometric-hint = · B to use {$type}
unlock-hint-account-list = ↑/↓ to navigate · Enter to confirm · Esc to cancel

### Onboarding
onboarding-create-account = Create Account
onboarding-enter-name = Please enter your account name (for local identification)
onboarding-enter-password = Set Master Password
onboarding-password-length = Master password must be at least 8 characters.
onboarding-confirm-password = Confirm Master Password
onboarding-confirm-desc = Please re-enter your master password to confirm.
onboarding-enter-hint = Password Hint (Optional)
onboarding-hint-desc = A hint to help you remember your password if forgotten. Can be left empty.
onboarding-confirm-title = Confirm Account Creation
onboarding-confirm-name = Account name: {$name}
onboarding-confirm-pw-masked = Master password: ******
onboarding-confirm-hint = Hint: {$hint}
onboarding-confirm-hint-none = (None)
onboarding-confirm-import-desc = Default templates will be imported, and you will enter the home page.
onboarding-exit-prompt = Exit account creation? Unsaved data will not be retained.

### Home
home-shortcut-list = List
home-shortcut-list-desc = List pages and objects
home-shortcut-search = Search
home-shortcut-search-desc = Global keyword search
home-shortcut-create = Create
home-shortcut-create-desc = Create a new object
home-shortcut-trash = Trash
home-shortcut-trash-desc = View deleted items
home-shortcut-settings = Settings
home-shortcut-settings-desc = Account preferences
home-shortcut-help = Help
home-shortcut-help-desc = View all commands
home-shortcut-plugins = Plugins
home-shortcut-plugins-desc = Browse plugin marketplace
home-hint = ↑/↓ Select, Enter to fill command, type /help for all commands

### Object list
object-list-title = Page List
object-list-empty = No content
object-list-table-id = ID
object-list-table-name = Name
object-list-table-type = Type
object-list-table-sensitivity = Sensitivity

### Object detail
object-detail-name = Name: {$name}
object-detail-id = ID: {$id}
object-detail-type = Type: {$type}
object-detail-section = Section: {$section}
object-detail-sensitivity = Sensitivity: {$level}
object-detail-version = Version: {$ver}
object-detail-sensitive-masked = Sensitive object: property values are masked. Edit mode can verify master password to view.

### Size / Stats
size-title = Account Statistics
size-pages = Pages: {$count}
size-objects = Objects: {$count}
size-trash = Trash items: {$count}
size-profiles = Profiles: {$count}
size-total-size = Total size: {$size}

### Search
search-title = Search "{$query}" · Scanned {$count} items
search-no-results = No matching results.

### History
history-title = History snapshots for {$id}
history-empty = No history snapshots.

### Trash
trash-title = Trash
trash-empty = Trash is empty.
trash-hint = ↑↓ Navigate · Space Select · R Restore · P Purge · Esc Back

### Backup
backup-title = Backup List
backup-empty = No backups.
backup-hint = Use /backup restore <id> to restore, /backup delete <id> to delete.
backup-created = Backup "{$name}" created ({$size})
backup-deleted = Backup deleted
backup-restore-success = Restored: {$id}

### Operation log
log-title = Audit Log · {$count} entries
log-empty = No audit log entries.
log-export-hint = Use /export_log [file name] to export the full log.

### Doctor
doctor-title = Diagnostic Report
doctor-data-dir = Data directory: {$path}
doctor-lock-status = Process lock: {$status}
doctor-lock-acquired = Acquired (GUI unavailable)
doctor-lock-none = Not exclusive
doctor-account-issues = Account issues:
doctor-accounts = Accounts: {$count}
doctor-account-count = Accounts: {$count}
doctor-core-version = Core library version: {$ver}
doctor-vault-version = Vault version: {$ver}
doctor-platform = Platform: {$os} / {$arch}
doctor-log-path = Log path: {$path}

### Settings menu
settings-title = Settings
settings-language = Language
settings-language-desc = Switch interface language
settings-theme = Theme
settings-theme-desc = Switch interface theme (System / Light / Dark)
settings-preference = Custom Preferences
settings-preference-desc = Write arbitrary key-value pairs to encrypted profile preferences
settings-debug-log = Export Debug Log
settings-debug-log-desc = Export audit log + sanitized system info to logs/

### Settings select
settings-current = Current
current-language = Language set to: {$code}
current-theme = Theme set to: {$code}

### Profile
profile-title = Profile
profile-id = ID: {$id}
profile-name = Name: {$name}
profile-version = Version: {$ver}
profile-updated = Updated: {$time}

### Template
template-title = Template Library
template-empty = No templates.
template-detail-title = Template Detail
template-hint = ↑↓ Navigate · Enter View · D Delete User Template · Esc Back

### Attachment
attachment-list-title = Attachments - {$id}
attachment-list-title-deleted = Attachments (with deleted) - {$id}
attachment-empty = No attachments.
attachment-hint = Use /attach add <path> to add, /attach delete <id> to delete, /attach purge <id> to purge.
attachment-added = Attachment added: {$path}
attachment-renamed = Renamed to: {$name}
attachment-deleted = Attachment deleted: {$id}
attachment-restored = Attachment restored: {$id}
attachment-purged = Attachment permanently deleted: {$id}
attachment-soft-delete-prompt = Soft delete attachment '{$id}'? It can be restored from trash.
attachment-purge-prompt = Permanently delete attachment '{$id}'? This cannot be undone.

### Sync
sync-title = Sync Status
sync-peers-from-vault = Persisted peers in vault (from historical sync sessions; does not include live mDNS discoveries)
sync-no-peers = No peers.
sync-unknown-peer = Unknown peer
sync-with-success = /sync with {$peer} completed: {$summary}. Detailed counts in audit log.
sync-with-failure = /sync with {$peer} failed: {$err}
sync-trusted = Peer {$id} marked as trusted
sync-untrusted = Peer {$id} marked as untrusted
sync-forgotten = Peer {$id} removed from vault
sync-need-unlock = Vault not unlocked. Please /unlock first.
sync-no-account = No current account.

### OCR
ocr-result-title = OCR · {$source}
ocr-no-text = (No text recognized)

### Help
help-title = Help
help-topic = Command: {$cmd}
help-unknown-topic = Unknown command. Use /help to see all commands.
help-header = Use /help <command> to see detailed usage.
biometric-generic-name = Biometrics

### About
about-title = About
about-version = Version: {$ver}
about-platform = Platform: {$os} / {$arch}
about-data-dir = Data directory: {$path}
about-lock = Process lock: {$status}
about-lock-acquired = Acquired (GUI unavailable)
about-lock-none = Not exclusive

### LLM
llm-config-title = LLM Configuration
llm-active-provider = Active: {$name}
llm-chat-ai-prefix = AI:
llm-chat-error-prefix = Error:
llm-conversation-empty = No conversations.
llm-stats-title = LLM Usage Statistics

### Plugin
plugin-list-title = Plugins ({$count})
plugin-detail-title = Plugin Detail: {$name}

### Embed Model
embed-model-title = Embedding Models
embed-model-install-hint = Use `/embed_model install <id>` to download and install from registry.
embed-model-registry-hint = Registry URL is controlled by the environment variable

### External editor
ext-editor-date = {$label} · Date
ext-editor-datetime = {$label} · Date & Time
ext-editor-time = {$label} · Time (HH:MM:SS)
ext-editor-select-action = {$label} · Select Action
ext-editor-edit-label = [Edit] {$i}. {$label}
ext-editor-delete-label = [Delete] {$i}. {$label}

### Status bar
status-locked = 🔒 Locked
status-unlocked = 🔓 Unlocked · {$id}
status-unlocked-with-name = 🔓 Unlocked · {$name} · {$id}
status-object = Object: {$name}
status-lock-countdown = Lock countdown: {$sec}s
status-settings = Settings · Lang={$lang} · Theme={$theme}

### Generic messages
loading = Loading...
unknown-command = Unknown command: {$cmd}
exit-prompt = Exit SoloSoul CLI?
session-timeout = Session timed out and locked.
no-results = No results.
