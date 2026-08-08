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
object-list-truncated = · truncated to first 200 results
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
template-field-count = Fields
doctor-source = Source: {$source}
profile-hint = Esc Back · /profile set <path> <value> to edit

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
generic-yes = Yes
generic-no = No
generic-none = None
loading = Loading...
unknown-command = Unknown command: {$cmd}
exit-prompt = Exit SoloSoul CLI?
session-timeout = Session timed out and locked.
no-results = No results.

### Command messages
cmd-no-previous-screen = No previous screen to go back.
cmd-no-accounts = No local accounts found.
cmd-already-logged-in = Already logged in.
cmd-no-accounts-gui = No local accounts found. Please use the GUI client to create one.
cmd-not-logged-in = Not logged in.
cmd-need-unlock = Please use /unlock to login.
cmd-vault-not-open = Vault is not open.
cmd-vault-locked = Vault is locked.
cmd-need-login = Not logged in.
cmd-no-pages = No pages. Please create one with /newpage.
cmd-no-current-account = No current account.
cmd-not-in-wizard = Not currently in a wizard.
cmd-no-changes = No changes to save.
cmd-page-not-found = Page '{$name}' not found.
cmd-object-not-found = Object '{$id}' not found or already deleted.
cmd-provide-object-id = Please provide an object ID, e.g. {$cmd} obj_xxx
cmd-provide-page-name = Please provide a page name, e.g. /newpage travel
cmd-provide-trash-id = Please provide a trash ID, e.g. /restore trash_xxx
cmd-profile-serialize-failed = Failed to serialize profile data: {$err}
cmd-unknown-subcommand = Unknown subcommand: {$cmd}
cmd-operation-failed = Operation failed: {$err}
cmd-saved = Saved
cmd-deleted = Deleted: {$name}
cmd-restored = Restored: {$id}
cmd-prompt-delete-page = Page '{$name}' contains {$count} objects. Deleting will move all to trash. Confirm?
cmd-page-deleted = Page '{$name}' and {$count} child objects deleted.
cmd-backup-created = Backup created: {$id} ({$size} profiles, {$bytes} bytes)
cmd-backup-restored = Backup '{$id}' restored
cmd-backup-deleted = Backup '{$id}' deleted
cmd-backup-name-empty = Backup name cannot be empty
cmd-trash-item-not-found = Trash item '{$id}' not found
cmd-purge-prompt = Permanently delete '{$name}'? This cannot be undone.
cmd-batch-restore-result = Restore complete: {$success} succeeded, {$failed} failed
cmd-restored-cascaded-page = Restored along with page "{$page}"
cmd-restored-cascaded-count = Restored page and {$count} object(s)
cmd-restored-rebuilt-page = Original page was permanently deleted; rebuilt page "{$page}" and restored object
cmd-batch-purge-result = Purge complete: {$success} succeeded, {$failed} failed
cmd-provide-object-id-or-detail = Please provide an object ID or run from the object detail page.
cmd-execute-in-detail = Please execute {$cmd} from the object detail page.
cmd-provide-attachment-id = Please provide an attachment ID, e.g. {$cmd} att_xxx
cmd-provide-filename = Please provide a new filename.
cmd-profile-rename-usage = Usage: /profile rename <name>
cmd-profile-set-usage = Usage: /profile set <path> <value>
cmd-profile-updated = Profile name updated to: {$name}
cmd-preference-updated = Preference updated: {$key}
cmd-setting-usage = Usage: /setting <key> <value>
cmd-provide-backup-id = Please provide a backup ID, e.g. /backup {$cmd} weekly_20260101_120000
cmd-backup-usage = Usage: /backup list | create <name> | restore <id> | delete <id>
cmd-export-import-usage = Usage: /export [options] | /import <path> [options]
cmd-provide-import-path = Please provide the file path to import.
cmd-import-success = Successfully imported {$count} objects.
cmd-password-min-length = Master password must be at least 8 characters.
cmd-password-mismatch = The two passwords entered do not match.
cmd-password-changed = Master password changed.
cmd-password-hint-updated = Password hint updated to: {$text}
cmd-password-hint-current = Current password hint: {$hint}
cmd-trash-retention-usage = Usage: /security trash-retention <days>
cmd-trash-retention-set = Trash retention period set to {$days} days.
cmd-biometric-not-supported = Biometrics not supported on this platform.
cmd-biometric-enabled = Biometric login enabled.
cmd-biometric-disabled = Biometric login disabled.
cmd-biometric-test-passed = Biometric test passed.
cmd-biometric-test-unavailable = Biometric test unavailable.
cmd-account-deleted = Account deleted.
cmd-password-wrong-canceled = Wrong password. Account deletion canceled.
cmd-verify-failed = Verification failed: {$err}
cmd-template-show-usage = Usage: /template show <id>
cmd-template-delete-usage = Usage: /template delete <id>
cmd-template-deleted = User template deleted: {$id}
cmd-load-failed = Failed to load: {$err}
cmd-export-import-unknown = Unknown export/import subcommand: {$cmd}
cmd-key-empty = Key cannot be empty.

### Security commands
cmd-security-usage = Usage: /security password|hint|trash-retention|delete-account|biometric
cmd-biometric-status = Biometrics: {$status} · {$configured} · Type: {$kind} · {$error}
cmd-biometric-usage = Usage: /security biometric status|enable|disable|test
prompt-current-password = Current Master Password
prompt-new-password = New Master Password
prompt-confirm-password = Confirm New Master Password
prompt-enable-biometric = Enter current master password to enable biometric
prompt-disable-biometric = Enter current master password to disable biometric
prompt-delete-account = Enter current master password to confirm account deletion
prompt-delete-account-confirm = ! Deleting your account will permanently erase all data. Continue?
prompt-export-password = Export Password
prompt-import-password = Import Password

### Export/Import
cmd-export-success = Exported {$count} objects to {$path}
cmd-import-preview = Package preview: v{$version}, {$count} objects, attachments: {$has}, password hint: {$hint}
cmd-export-password-too-short = Export password must be at least 8 characters.
cmd-export-password-format = Export password must contain both letters and digits.
cmd-export-password-same-master = Export password cannot be the same as the master password.
cmd-export-password-verify-failed = Failed to verify master password: {$err}
cmd-export-too-many-args = Too many file arguments.
cmd-export-no-scope = Please specify one of --full, --pages, or --objects.
cmd-import-need-strategy = --strategy requires a strategy value (skip/overwrite/merge).
cmd-account-not-found = No current account.

### LLM commands
cmd-llm-need-login = Not logged in. Cannot view LLM configuration.
cmd-llm-vault-locked = Vault is not unlocked.
cmd-llm-no-active-provider = No active LLM provider. Use /llm_config to configure.
cmd-llm-config-failed = Failed to load LLM configuration: {$err}
cmd-llm-stats-failed = Failed to load LLM statistics: {$err}
cmd-llm-list-failed = Failed to load conversation list: {$err}
cmd-llm-need-login-chat = Not logged in. Cannot use LLM chat.
cmd-llm-loaded-conversation = Loaded conversation: {$name}

### Sync commands
cmd-sync-usage = Usage: /sync <subcommand> [args]
cmd-sync-trust-usage = Usage: /sync trust <peer>
cmd-sync-untrust-usage = Usage: /sync untrust <peer>
cmd-sync-forget-usage = Usage: /sync forget <peer>
cmd-sync-with-usage = Usage: /sync with <peer-or-host:port>
cmd-sync-runtime-failed = Failed to create async runtime: {$err}
cmd-sync-trust-operation-failed = /sync trust operation failed: {$err}
cmd-sync-forget-operation-failed = /sync forget operation failed: {$err}
cmd-sync-info = Persisted peers from historical sync sessions

### OCR commands
cmd-ocr-usage = Usage: /ocr scan [--mrz] <image-path>
cmd-ocr-unknown-flag = /ocr scan: unknown flag {$flag}. Usage: /ocr scan [--mrz] <image-path>
cmd-ocr-extra-arg = /ocr scan: rejecting extra argument {$arg}. Usage: /ocr scan [--mrz] <image-path>
cmd-ocr-image-not-found = Image not found: {$path}
cmd-ocr-env-parse-failed = SOLOSOUL_OCR_TIER parse failed: {$err}
cmd-ocr-tier-not-installed = {$tier} tier model not installed. Please install via GUI or place at {$path}.
cmd-ocr-engine-failed = Failed to load OCR engine: {$err}
cmd-ocr-mrz-not-found = No MRZ region detected in image.
cmd-ocr-mrz-failed = MRZ scan failed: {$err}
cmd-ocr-scan-failed = OCR scan failed: {$err}

### Embed Model commands
cmd-embed-usage = Usage: /embed_model install <model_id>
cmd-embed-remove-usage = Usage: /embed_model remove <model_id>
cmd-embed-already-installed = Model {$model} already installed.
cmd-embed-not-installed = Model {$model} not installed.
cmd-embed-installed = Embedding model {$model} installed. Run /embed_model list to view.
cmd-embed-install-failed = /embed_model install failed: {$err}
cmd-embed-removed = Embedding model {$model} removed.
cmd-embed-remove-failed = /embed_model remove failed: {$err}
cmd-embed-runtime-failed = Failed to create async runtime: {$err}

### Plugin commands
cmd-plugin-usage-run = Usage: /plugin_run <plugin_id>
cmd-plugin-usage-install = Usage: /plugin_install <plugin_id>
cmd-plugin-usage-update = Usage: /plugin_update <plugin_id>
cmd-plugin-usage-uninstall = Usage: /plugin_uninstall <plugin_id>
cmd-plugin-usage-search = Usage: /plugin_search <keyword>
cmd-plugin-invalid-id = Invalid plugin ID: {$id} (only letters, digits, _, - and . allowed)

### App
app-field-password-verify = Field [{$field}] is {$level} level. Please enter your master password to verify.
app-password-verify-failed = Master password verification failed.
app-password-verify-error = Verification failed: {$err}
app-session-timeout-locked = Session timed out and was locked.
app-biometric-not-enabled = Biometric login is not enabled for the current account.
app-account-name-empty = Account name cannot be empty.
app-password-too-short = Master password must be at least 8 characters.
app-password-mismatch = The two passwords do not match. Please try again.
app-exit-wizard-confirm = Exit account creation? Unsaved data will be lost.
app-exit-cli-confirm = Exit SoloSoul CLI?
app-account-create-failed = Account creation failed: {$err}
app-login-failed = Login failed: {$err}
app-biometric-unlock-failed = Biometric unlock failed: {$err}
app-unknown-command = Unknown command: {$cmd}
app-not-in-wizard = Not in a wizard.
app-save-failed = Save failed: {$err}
app-no-changes-to-save = There are no changes to save.
app-object-name = Object name
app-vault-locked = Vault is not unlocked.
app-unlock-vault-title = Unlock SoloSoul Vault
app-editing-preferences = Editing custom preferences...
app-error-overlay = ! Error
app-info-overlay = ℹ Info
app-esc-to-close = Press Esc to close
cmd-plugin-market-empty = No plugins available in the marketplace.
cmd-plugin-need-login = Not logged in. Cannot run plugins.
cmd-plugin-vault-locked = Vault is not unlocked.
cmd-plugin-not-found = Plugin not found: {$id}
cmd-plugin-running = Running plugin: {$id} ...
cmd-plugin-run-result = Plugin {$id} completed: exit_code={$code}, fuel={$fuel}
cmd-plugin-run-failed = Plugin {$id} run failed: {$err}
cmd-plugin-installed = Plugin {$id} v{$ver} installed successfully.
cmd-plugin-install-failed = Failed to install plugin {$id}: {$err}
cmd-plugin-updated = Plugin {$id} updated to v{$ver}.
cmd-plugin-update-failed = Failed to update plugin {$id}: {$err}
cmd-plugin-uninstalled = Plugin {$id} uninstalled.
cmd-plugin-uninstall-failed = Failed to uninstall plugin {$id}: {$err}
cmd-plugin-no-sessions = No active plugin sessions.
cmd-plugin-sessions-header = Active sessions ({$count}):
cmd-plugin-list-sessions-failed = Failed to list sessions: {$err}
cmd-plugin-none-installed = No plugins installed locally.
cmd-plugin-installed-header = Installed plugins ({$count}):
cmd-plugin-list-installed-failed = Failed to list installed plugins: {$err}
cmd-plugin-limit-must-be-positive = Limit must be a positive integer.
cmd-plugin-limit-must-be-number = Limit must be a number.
cmd-plugin-no-audit-logs = No plugin audit logs.
cmd-plugin-audit-header = Audit Log (recent {$count}):
cmd-plugin-audit-failed = Failed to get audit log: {$err}
cmd-plugin-updating-registry = Updating plugin registry...
cmd-plugin-registry-updated = Plugin registry updated.
cmd-plugin-registry-update-failed = Failed to update registry: {$err}
cmd-plugin-search-no-match = No plugins matching "{$keyword}" found.
cmd-plugin-search-failed = Plugin search failed: {$err}
cmd-plugin-init-failed = Failed to initialize plugin manager: {$err}

### History
cmd-history-usage = Please provide an object ID, e.g. /history obj_xxx
cmd-rollback-usage = Please provide an object ID and snapshot ID, e.g. /rollback obj_xxx snap_xxx
cmd-rollback-confirm = Confirm rollback of '{$obj}' to snapshot '{$snap}'? Unsaved changes will be lost.
cmd-rollback-complete = Object '{$name}' rolled back to snapshot '{$snap}'

### Search
cmd-search-need-keyword = Please provide a search keyword, e.g. /search passport

### Template
cmd-template-usage = Usage: /template | /template show <id> | /template delete <id>
cmd-template-source-user = User
cmd-template-source-system = System
cmd-template-delete-failed = Delete failed: {$err}

### Profile
cmd-profile-usage = Usage: /profile | /profile rename <name> | /profile set <path> <value>

### Log
cmd-log-serialize-failed = Failed to serialize log: {$err}
cmd-log-dir-failed = Failed to create log directory: {$err}
cmd-log-write-failed = Failed to write export file: {$err}
cmd-log-exported = Audit log exported to: {$path}
cmd-log-vault-not-open = Vault is not open.

### Doctor
doctor-dir-status-label = Data directory status:
doctor-dir-writable-label = Data directory writable:
doctor-lock-status-label = Process lock status:
cmd-doctor-no-issues = No issues found.

### New Object
newobj-create-object = Create Object
newobj-select-page = Create Object: Select Page
newobj-select-page-desc = Select a page as the parent for the new object, or press q to cancel.
newobj-no-pages = No pages. Please create one with /newpage first.
newobj-hint-up-down-enter-q = ↑/↓ Select · Enter Confirm · q Cancel
newobj-blank-object = Blank Object
newobj-select-template = Create Object: Select Template (Page:
newobj-select-template-desc = Select a template to fill in fields, or select "Blank Object" to enter only a name.
newobj-fields = Fields
newobj-fill-fields-hint = Press Enter to edit fields, s to save, q to cancel.
newobj-fields-nav = ↑/↓ Select Field · Enter Edit · n Rename · s Save · q Cancel

### Edit Object
editobj-object-info = Object Info
editobj-properties = Properties
editobj-template = Template: {$tpl}


### Settings select
settings-lang-title = Select Language
settings-theme-title = Select Theme
settings-select-hint = ↑/↓ Select · Enter or click to apply · Esc Cancel

### Sync
sync-peers-title = Peer List
sync-no-peers-hint = Tip: Use `/sync with <host:port>` to sync with a GUI instance. Persistent peers will appear here.
sync-subcommand-prefix = Subcommands
sync-peers-hint-p1 = To sync with a peer, first run
sync-peers-hint-p2 =  then enable continuous sync via the GUI.

### OCR
ocr-tiers = Model Tiers
ocr-recognized-text = Recognized Text
ocr-hint-prefix = Tip
ocr-hint-text = Switch tiers with `SOLOSOUL_OCR_TIER=small|medium|tiny /ocr scan <path>`; if a model is not installed, the GUI or manual placement will be prompted.
ocr-mrz-hint-text = MRZ fields come from the machine-readable zone at the bottom of the image; checksum validation is performed by the MRZ parser.
ocr-mrz-fields = MRZ Fields

### Embed Model
embed-model-not-installed = (No local embedding models installed)
embed-model-list-title = Model List

### LLM
llm-active-label = Active:
llm-providers = Providers
llm-stats-summary = Total requests: {$count}  |  Total tokens: {$tokens}  |  Prompt: {$prompt}  |  Completion: {$completion}
llm-stats-model = Model
llm-stats-provider = Provider
llm-stats-count = Count
llm-stats-tokens = Tokens
llm-stats-by-model = By Model
llm-chat-welcome = LLM Chat ready. Type your question and press Enter to send. Type /back to return.
llm-chat-title = LLM Chat
llm-chat-user-prefix = You:
llm-chat-thinking = AI: ⏳ Thinking...
llm-chat-waiting = ⏳ Waiting for response...
llm-chat-input-hint = Type message, Enter to send, Esc to return
llm-conversation-title = LLM Conversation History
llm-conversation-count-prefix = Total 
llm-conversations-label = Conversations
hint-up-down-esc-q = ↑/↓ Navigate  Esc/q Return
hint-esc-q = Esc/q Return
hint-up-down-enter-esc-q =  conversations  |  ↑/↓ Select  Enter Open  Esc/q Return

### Plugin List
plugin-list-empty = No plugins available.
plugin-list-no-match = No matching results.
plugin-list-hint = ↑/↓ Navigate  Type to filter  |  Enter Detail  r Run  i Install  u Update  d Uninstall  |  q/Esc Back
plugin-list-hint-filtering = Type keyword to filter  Esc Clear  Backspace Delete  |  ↑/↓ Navigate  Enter Detail  r Run

### Plugin Detail
plugin-detail-prefix = Plugin Detail:
plugin-detail-author = Author:
plugin-detail-category = Category:
plugin-detail-tier = Tier:
plugin-detail-confirm = Requires confirmation:
plugin-detail-homepage = Homepage:
plugin-detail-core = Requires Core:
plugin-detail-permissions = Permissions:
plugin-detail-no-permissions =  No special permissions
plugin-detail-network = Network Policy:
plugin-detail-params = Parameters:
plugin-detail-required = [Required]
plugin-detail-optional = [Optional]
plugin-detail-wasm-hash = WASM SHA256:
plugin-detail-ttl = Data TTL:


### Status bar (additional)
status-welcome = Not logged in · No account
status-account-list = Account List
status-unlock = Login
status-size = Account Statistics
status-doctor = Doctor
status-new-object = Create Object Wizard
status-edit-object = Edit Object Wizard
status-trash = Trash
status-onboarding = Create Account
status-search = Search Results
status-history = History Snapshots
status-operation-log = Audit Log
status-about = About
status-help = Help
status-attachment = Attachments
status-backup = Backup List
status-profile = Profile
status-template-list = Template List
status-template-detail = Template Detail
status-llm-config = LLM Configuration
status-llm-stats = LLM Statistics
status-conversation = Conversation History
status-llm-chat = LLM Chat
status-plugin-list = Plugin List
status-plugin-detail = Plugin Detail
status-sync = Device Sync
status-ocr = OCR
status-embed = Embedding Models
status-settings-language = Settings · Language
status-settings-theme = Settings · Theme
status-settings-preference = Settings · Custom Preferences
status-quit = Exiting
status-lock-held = [L] Process lock held · GUI unavailable
status-lock-not-exclusive = [!] Not exclusive


### Commands Phase 3
cmd-attachment-usage = Usage: /attach list [object_id] | add <file_path> | rename <id> <new_name> | delete <id> | restore <id> | purge <id> | cleanup
cmd-provide-file-path = Please provide a file path, e.g. /attach add /path/to/file.pdf
cmd-provide-attachment-id-example = Please provide an attachment ID, e.g. /attach rename att_xxx new.pdf
cmd-prompt-soft-delete-attachment = Soft delete attachment '{$id}'? Can be restored from trash.
cmd-prompt-purge-attachment = Permanently delete attachment '{$id}'? This cannot be undone.
cmd-cleanup-result = Cleanup complete: removed {$count} orphan attachments, freed {$bytes} bytes
cmd-prompt-restore-backup =
 Confirm restore backup '{$id}'?
 Created: {$date}
 Contains {$count} profile(s).
 Profiles with the same name in the current Vault will be overwritten.
cmd-prompt-delete-backup =
 Confirm delete backup '{$id}'?
 This cannot be undone.
cmd-llm-current-model =
 Current model: {$name} - {$model}
 Provider: {$url}
 API type: {$api_type}
cmd-template-load-failed = Failed to load system templates: {$err}
cmd-language-set = Language set to: {$code}
cmd-theme-set = Theme set to: {$name}
cmd-preference-value-label = Preference value (key={$key}, JSON will be parsed, otherwise saved as string)
cmd-debug-log-exported = Debug log exported to: {$path}
cmd-ocr-no-models =
 Model directory: {$path}
 No models installed. Please install from GUI or download to this directory.
cmd-ocr-models-status =
 Model directory: {$path}
 Installed: {$installed}
cmd-ocr-status-title = OCR Status ({$path} directory)
cmd-extra-file-arg = Extra file argument
cmd-export-need-scope = Please specify one of: --full, --pages, or --objects
cmd-import-need-strategy-value = --strategy requires a strategy value
cmd-export-password-complexity = Export password must contain both letters and digits
cmd-account-not-found-generic = Current account not found
cmd-export-password-same-as-master = Export password cannot be the same as the master password
cmd-verify-master-failed = Failed to verify master password: {$err}
