//! 全局状态机与 App 状态。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;
use solosoul_core::process_lock::ProcessLock;
use solosoul_core::{AccountSummary, ObjectRecord, ObjectSummary, UserTemplate, VaultService};
use zeroize::Zeroizing;

use crate::commands;
use crate::commands::search::SearchResultItem;
use crate::widgets::command_input::CommandInput;
use crate::widgets::command_palette::{CommandPalette, PaletteAction};
use crate::widgets::field_editor::{self, EditableField};
use crate::widgets::password_input::PasswordInput;
use crate::widgets::prompt::{self, PromptResult, PromptSpec};

/// 回收站筛选条件。
#[derive(Debug, Clone, Default)]
pub struct TrashFilter {
    pub item_type: Option<String>,
    pub since_ms: Option<i64>,
    pub search: Option<String>,
}

/// 账户统计报告。
#[derive(Debug, Clone, Default)]
pub struct SizeReport {
    pub page_count: usize,
    pub object_count: usize,
    pub trash_count: usize,
    pub profile_count: usize,
    pub total_size_bytes: u64,
}

/// 可点击区域动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickAction {
    /// 执行一条 CLI 命令。
    Command(&'static str),
    /// 进入首次创建账户向导。
    StartOnboarding,
}

/// 可点击区域。
#[derive(Debug, Clone)]
pub struct ClickableRegion {
    pub rect: Rect,
    pub action: ClickAction,
}

/// 当前所处界面/阶段。
// 状态机变体大小差异大是预期内（ObjectRecord 较大），可读性优先。
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AppPhase {
    /// 无本地账户：提示用户先用 GUI 创建账户
    Welcome,
    /// 有账户但未登录：显示可用命令列表
    Locked,
    /// /account_list 结果页
    AccountList { accounts: Vec<AccountSummary> },
    /// 登录向导
    UnlockWizard { step: UnlockStep },
    /// 已登录：主界面
    Home { account_id: String },
    /// 页面/对象列表结果页
    ObjectList {
        items: Vec<ObjectSummary>,
        title: String,
    },
    /// 对象详情页
    ObjectDetail { object: ObjectRecord },
    /// 账户统计页
    Size { report: SizeReport },
    /// /doctor 结果页
    Doctor {
        report: commands::doctor::DoctorReport,
    },
    /// 创建对象向导
    NewObjectWizard { step: NewObjectStep },
    /// 编辑对象向导
    EditObjectWizard {
        object_id: String,
        step: EditObjectStep,
    },
    /// 回收站列表
    TrashList {
        items: Vec<solosoul_core::TrashItemSummary>,
        selected: usize,
        selected_ids: Vec<String>,
        filter: TrashFilter,
    },
    /// 首次启动创建账户向导
    Onboarding { step: OnboardingStep },
    /// 搜索结果页
    SearchResults {
        query: String,
        items: Vec<SearchResultItem>,
        selected: usize,
        truncated: bool,
        total_scanned: usize,
    },
    /// 对象历史快照页
    HistoryList {
        object_id: String,
        snapshots: Vec<serde_json::Value>,
        selected: usize,
    },
    /// 审计日志页
    OperationLog {
        account_id: String,
        entries: Vec<solosoul_core::AuditLogEntry>,
        selected: usize,
    },
    /// 关于页
    About { info: commands::system::AboutInfo },
    /// 帮助页
    Help { topic: Option<String> },
    /// 附件列表页
    AttachmentList {
        object_id: String,
        items: Vec<crate::commands::attachment::AttachmentMeta>,
        show_deleted: bool,
        selected: usize,
    },
    /// 备份列表页
    BackupList {
        items: Vec<crate::commands::backup::BackupInfo>,
        selected: usize,
    },
    /// Profile 展示页
    Profile {
        profile: solosoul_core::Profile,
        data: serde_json::Value,
        selected: usize,
    },
    /// 模板列表页
    TemplateList {
        user_templates: Vec<solosoul_core::UserTemplate>,
        system_templates: Vec<solosoul_core::template_service::SystemTemplate>,
        selected: usize,
    },
    /// 模板详情页
    TemplateDetail {
        template_id: String,
        name: String,
        source: String,
        json: String,
    },
    /// 安全退出
    Quit,
}

/// 创建对象向导步骤。
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NewObjectStep {
    /// 选择父页面
    SelectPage {
        pages: Vec<ObjectSummary>,
        selected: usize,
    },
    /// 选择模板
    SelectTemplate {
        page_id: String,
        page_name: String,
        templates: Vec<UserTemplate>,
        selected: usize,
    },
    /// 填写字段
    FillFields {
        page_id: String,
        page_name: String,
        template: Option<UserTemplate>,
        name: String,
        fields: Vec<EditableField>,
        selected: usize,
    },
}

/// 编辑对象向导步骤。
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EditObjectStep {
    /// 字段概览
    Overview {
        object: ObjectRecord,
        fields: Vec<EditableField>,
        selected: usize,
    },
}

/// 首次启动创建账户向导步骤。
#[derive(Debug, Clone)]
pub enum OnboardingStep {
    /// 输入账户名
    EnterName,
    /// 输入主密码
    EnterPassword { name: String },
    /// 确认主密码
    ConfirmPassword {
        name: String,
        password: Zeroizing<String>,
    },
    /// 输入密码提示词
    EnterHint {
        name: String,
        password: Zeroizing<String>,
    },
    /// 最终确认
    Confirm {
        name: String,
        password: Zeroizing<String>,
        hint: Option<String>,
    },
}

/// 登录向导步骤。
#[derive(Debug, Clone)]
pub enum UnlockStep {
    /// 选择账户
    SelectAccount {
        accounts: Vec<AccountSummary>,
        selected: usize,
    },
    /// 输入密码
    EnterPassword {
        account_id: String,
        account_name: String,
        password_hint: Option<String>,
        biometric_configured: bool,
        biometry_type: Option<String>,
    },
}

/// 需要临时退出 ratatui 全屏，调用 inquire 进行编辑的字段请求。
#[derive(Debug, Clone)]
pub enum ExternalEditRequest {
    Date(EditableField),
    DateTime(EditableField),
    Select(EditableField),
    MultiSelect(EditableField),
    Textarea(EditableField),
}

pub struct App {
    pub phase: AppPhase,
    /// 返回 `Locked` 或 `Welcome` 时恢复此上一屏
    pub previous_phase: Option<AppPhase>,
    pub vault_service: Arc<VaultService>,
    pub process_lock: Option<ProcessLock>,
    pub command_input: CommandInput,
    pub password_input: PasswordInput,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    pub last_activity: Instant,
    pub auto_lock_duration: Duration,
    /// 暂停自动锁定计时（模态提示或阻塞操作期间）
    pub auto_lock_paused: bool,
    /// 当前模态提示
    pub prompt: Option<prompt::PromptState>,
    /// 全局错误/消息 overlay，按任意键或 Esc 清除
    pub error_message: Option<String>,
    /// 日志文件路径，/doctor 中展示
    pub log_path: PathBuf,
    /// 当前账户显示名称（用于首页与状态栏）
    pub account_name: String,
    /// 首页快捷卡片的当前焦点索引。
    pub selected_shortcut: usize,
    /// 斜杠命令面板状态。
    pub command_palette: CommandPalette,
    /// 当前帧可点击区域（由渲染阶段写入，鼠标事件读取）。
    pub clickable_regions: Vec<ClickableRegion>,
    /// 当前鼠标位置（用于悬停高亮）。
    pub mouse_pos: Option<(u16, u16)>,
    /// Logo sheen 动画偏移量。
    pub sheen_offset: u16,
    /// 锁定页当前选中的动作按钮索引。
    pub locked_selected: usize,
    /// 欢迎页当前选中的选项索引。
    pub welcome_selected: usize,
    /// 当前触屏/鼠标拖拽起始位置，用于在支持滚动的页面区分点击与滑动滚动。
    pub drag_start: Option<(u16, u16)>,
    /// 待通过 inquire 在外部编辑的字段请求。
    pub external_edit: Option<ExternalEditRequest>,
}

impl App {
    pub fn new(vault_service: Arc<VaultService>) -> Result<Self> {
        let base_path = vault_service.base_path().to_path_buf();

        // 获取进程级排他锁
        let process_lock = match ProcessLock::acquire(&base_path) {
            Ok(lock) => Some(lock),
            Err(e) => {
                // Phase 1：仅记录警告，不阻塞启动（便于 doctor 展示状态）
                tracing::warn!("无法获取进程锁: {}", e);
                None
            }
        };

        let log_path = base_path.join("logs").join("cli.log");

        let phase = if vault_service.has_any_account() {
            AppPhase::Locked
        } else {
            AppPhase::Welcome
        };

        Ok(Self {
            phase,
            previous_phase: None,
            vault_service,
            process_lock,
            command_input: CommandInput::new(),
            password_input: PasswordInput::new(),
            command_history: Vec::new(),
            history_index: None,
            last_activity: Instant::now(),
            auto_lock_duration: Duration::from_secs(5 * 60),
            auto_lock_paused: false,
            prompt: None,
            error_message: None,
            log_path,
            account_name: String::new(),
            selected_shortcut: 0,
            command_palette: CommandPalette::new(),
            clickable_regions: Vec::new(),
            mouse_pos: None,
            sheen_offset: 0,
            locked_selected: 0,
            welcome_selected: 0,
            drag_start: None,
            external_edit: None,
        })
    }

    /// 根据 account_id 查找账户显示名称。
    fn lookup_account_name(&self, account_id: &str) -> String {
        self.vault_service
            .list_accounts()
            .into_iter()
            .find(|a| a.id == account_id)
            .map(|a| a.name)
            .unwrap_or_else(|| account_id.to_string())
    }

    /// 进入已登录首页，并刷新当前账户显示名称。
    fn enter_home(&mut self, account_id: impl AsRef<str>) {
        let account_id = account_id.as_ref().to_string();
        self.account_name = self.lookup_account_name(&account_id);
        self.selected_shortcut = 0;
        self.phase = AppPhase::Home { account_id };
    }
}

/// 根据字段类型构造对应的外部编辑请求。
pub fn build_external_edit_request(field: EditableField) -> ExternalEditRequest {
    use solosoul_core::PropertyType;
    match field.prop_type {
        PropertyType::Date => ExternalEditRequest::Date(field),
        PropertyType::DateTime => ExternalEditRequest::DateTime(field),
        PropertyType::Select => ExternalEditRequest::Select(field),
        PropertyType::MultiSelect => ExternalEditRequest::MultiSelect(field),
        PropertyType::MultilineText => ExternalEditRequest::Textarea(field),
        _ => ExternalEditRequest::Textarea(field),
    }
}

impl App {
    /// 将外部编辑器（inquire）返回的值应用到当前向导的选中字段。
    #[allow(clippy::collapsible_match)]
    pub fn apply_external_edit(&mut self, value: Option<serde_json::Value>) {
        if value.is_none() {
            return;
        }
        let value = value.unwrap();
        match &mut self.phase {
            AppPhase::NewObjectWizard {
                step:
                    NewObjectStep::FillFields {
                        fields, selected, ..
                    },
            } => {
                if *selected < fields.len() {
                    fields[*selected].value = value;
                }
            }
            AppPhase::EditObjectWizard {
                step:
                    EditObjectStep::Overview {
                        object,
                        fields,
                        selected,
                        ..
                    },
                ..
            } => {
                if *selected < fields.len() {
                    let key = fields[*selected].key.clone();
                    fields[*selected].value = value.clone();
                    if let serde_json::Value::Object(ref mut map) = object.properties {
                        map.insert(key, value);
                    }
                }
            }
            _ => {}
        }
    }

    /// 开始编辑字段；若字段敏感则先弹出主密码验证。
    pub fn start_field_edit(&mut self, field: EditableField) {
        if field.requires_password_verification() {
            self.prompt_verify_password_for_field(field);
        } else {
            self.proceed_with_field_edit(field);
        }
    }

    /// 弹出主密码验证，通过后再进入字段编辑。
    fn prompt_verify_password_for_field(&mut self, field: EditableField) {
        let account_id = self.vault_service.get_current_account().unwrap_or_default();
        prompt::open(
            self,
            PromptSpec::Text {
                label: format!(
                    "字段 [{}] 为 {} 级别，请输入主密码验证",
                    field.label,
                    field.sensitivity.to_uppercase()
                ),
                initial: String::new(),
                mask: true,
                allow_toggle_mask: false,
            },
            Box::new(move |app, result| {
                if let PromptResult::Text(password) = result {
                    match app.vault_service.verify_password(&account_id, &password) {
                        Ok(true) => app.proceed_with_field_edit(field),
                        Ok(false) => {
                            app.error_message = Some("主密码验证失败".to_string());
                        }
                        Err(e) => {
                            app.error_message = Some(format!("验证失败: {}", e));
                        }
                    }
                }
            }),
        );
    }

    /// 验证已通过，根据字段类型进入内部或外部编辑器。
    fn proceed_with_field_edit(&mut self, field: EditableField) {
        if field_editor::needs_external_editor(&field) {
            self.external_edit = Some(build_external_edit_request(field));
            return;
        }
        self.open_internal_field_editor(field);
    }

    /// 打开自绘字段编辑器（适用于 text/number/boolean 等简单类型）。
    fn open_internal_field_editor(&mut self, field: EditableField) {
        use crate::widgets::prompt::PromptResult;

        let spec = field_editor::prompt_for_field(&field);
        let is_critical = field.is_critical();
        let field_key = field.key.clone();
        let field_label = field.label.clone();

        match self.phase.clone() {
            AppPhase::NewObjectWizard {
                step:
                    NewObjectStep::FillFields {
                        page_id,
                        page_name,
                        template,
                        name,
                        mut fields,
                        selected,
                    },
            } => {
                prompt::open(
                    self,
                    spec,
                    Box::new(move |app: &mut App, result: PromptResult| {
                        if let Some(new_value) = field_editor::value_from_result(&result, &field) {
                            if is_critical {
                                app.log_critical_field_edit(&field_key, &field_label);
                            }
                            if selected < fields.len() {
                                fields[selected].value = new_value;
                            }
                        }
                        app.phase = AppPhase::NewObjectWizard {
                            step: NewObjectStep::FillFields {
                                page_id,
                                page_name,
                                template,
                                name,
                                fields,
                                selected,
                            },
                        };
                    }),
                );
            }
            AppPhase::EditObjectWizard {
                object_id,
                step:
                    EditObjectStep::Overview {
                        mut object,
                        mut fields,
                        selected,
                    },
            } => {
                prompt::open(
                    self,
                    spec,
                    Box::new(move |app: &mut App, result: PromptResult| {
                        if let Some(new_value) = field_editor::value_from_result(&result, &field) {
                            if is_critical {
                                app.log_critical_field_edit(&field_key, &field_label);
                            }
                            if selected < fields.len() {
                                let key = fields[selected].key.clone();
                                fields[selected].value = new_value.clone();
                                if let serde_json::Value::Object(ref mut map) = object.properties {
                                    map.insert(key, new_value);
                                }
                            }
                        }
                        app.phase = AppPhase::EditObjectWizard {
                            object_id,
                            step: EditObjectStep::Overview {
                                object,
                                fields,
                                selected,
                            },
                        };
                    }),
                );
            }
            _ => {}
        }
    }

    /// 记录 critical 字段编辑审计日志。
    fn log_critical_field_edit(&self, field_key: &str, field_label: &str) {
        if let (Some(account_id), Some(vault)) = (
            self.vault_service.get_current_account(),
            self.vault_service.get_vault_store(),
        ) {
            let _ = vault.log_structured(
                "critical_field_edit",
                "field",
                Some(field_key),
                Some(field_label),
                &account_id,
                Some("cli"),
            );
        }
    }

    /// 处理事件，返回 true 表示应退出事件循环。
    pub fn handle_event(&mut self, event: crate::events::Event) -> Result<bool> {
        match event {
            crate::events::Event::Key(key) => {
                self.last_activity = Instant::now();
                self.handle_key(key)
            }
            crate::events::Event::Mouse(mouse) => {
                self.last_activity = Instant::now();
                self.handle_mouse(mouse)
            }
            crate::events::Event::Tick => self.handle_tick(),
        }
    }

    fn handle_tick(&mut self) -> Result<bool> {
        // 自动锁定检测（模态提示期间暂停）
        if self.auto_lock_paused {
            self.last_activity = Instant::now();
        }
        if self.vault_service.is_unlocked() {
            let idle = Instant::now().duration_since(self.last_activity);
            if idle >= self.auto_lock_duration {
                self.vault_service.lock();
                self.password_input.clear();
                self.prompt = None;
                self.auto_lock_paused = false;
                self.phase = AppPhase::Locked;
                self.error_message = Some("会话已超时锁定".to_string());
            }
        }

        // Logo 扫光动画偏移
        self.sheen_offset = self
            .sheen_offset
            .wrapping_add(crate::screens::logo::SHEEN_STEP);

        Ok(false)
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // 全局 Esc：先清 error overlay
        if key.code == KeyCode::Esc && self.error_message.take().is_some() {
            return Ok(false);
        }

        // 模态提示优先消费事件
        if self.prompt.is_some() {
            return Ok(prompt::handle_key(self, key));
        }

        // 根据当前阶段分发
        match &self.phase.clone() {
            AppPhase::Onboarding { step } => self.handle_onboarding_key(key, step.clone()),
            AppPhase::UnlockWizard { step } => self.handle_unlock_key(key, step.clone()),
            AppPhase::NewObjectWizard { step } => self.handle_new_object_key(key, step.clone()),
            AppPhase::EditObjectWizard { object_id, step } => {
                self.handle_edit_object_key(key, object_id.clone(), step.clone())
            }
            AppPhase::SearchResults { .. } => self.handle_search_results_key(key),
            AppPhase::HistoryList { .. } => self.handle_history_list_key(key),
            AppPhase::AttachmentList { .. } => self.handle_attachment_list_key(key),
            AppPhase::BackupList { .. } => self.handle_backup_list_key(key),
            AppPhase::Profile { .. } => self.handle_profile_key(key),
            AppPhase::TemplateList { .. } => self.handle_template_list_key(key),
            AppPhase::TemplateDetail { .. } => self.handle_template_detail_key(key),
            AppPhase::TrashList { .. } => self.handle_trash_list_key(key),
            AppPhase::Locked => self.handle_locked_key(key),
            AppPhase::Welcome => self.handle_welcome_key(key),
            _ => self.handle_command_key(key),
        }
    }

    /// 鼠标事件处理（移动更新悬停位置，左键单击执行动作，滚轮/拖拽滚动选项列表）。
    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<bool> {
        let pos = (mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::Moved => {
                self.mouse_pos = Some(pos);
                Ok(false)
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.has_scrollable_options() {
                    // 可滚动选项页：先记录拖拽起点，区分手指点按与滑动滚动。
                    self.drag_start = Some(pos);
                    Ok(false)
                } else {
                    self.execute_click(pos)
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.mouse_pos = Some(pos);
                if self.has_scrollable_options() {
                    if let Some(start) = self.drag_start {
                        const DRAG_THRESHOLD: i32 = 2;
                        let dy = pos.1 as i32 - start.1 as i32;
                        if dy.abs() >= DRAG_THRESHOLD {
                            let steps = (dy / DRAG_THRESHOLD).clamp(-1, 1);
                            self.scroll_selection(steps);
                            self.drag_start = Some(pos);
                        }
                    }
                }
                Ok(false)
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let result = if self.has_scrollable_options() {
                    if let Some(start) = self.drag_start {
                        const TAP_THRESHOLD: i32 = 2;
                        let dx = pos.0 as i32 - start.0 as i32;
                        let dy = pos.1 as i32 - start.1 as i32;
                        if dx.abs() < TAP_THRESHOLD && dy.abs() < TAP_THRESHOLD {
                            self.execute_click(pos)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                };
                self.drag_start = None;
                result
            }
            MouseEventKind::ScrollDown => {
                if self.has_scrollable_options() {
                    self.scroll_selection(1);
                }
                Ok(false)
            }
            MouseEventKind::ScrollUp => {
                if self.has_scrollable_options() {
                    self.scroll_selection(-1);
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// 当前阶段是否使用可滚动选项列表。
    fn has_scrollable_options(&self) -> bool {
        matches!(
            self.phase,
            AppPhase::Locked | AppPhase::Welcome | AppPhase::Home { .. }
        )
    }

    /// 在可点击区域上执行点击动作。
    fn execute_click(&mut self, pos: (u16, u16)) -> Result<bool> {
        if let Some(region) = self
            .clickable_regions
            .iter()
            .find(|r| r.rect.contains(pos.into()))
            .cloned()
        {
            match region.action {
                ClickAction::Command(cmd) => {
                    self.command_input.set_value(cmd.to_string());
                    self.execute_command()
                }
                ClickAction::StartOnboarding => {
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterName,
                    };
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// 滚动当前阶段的可滚动选项选择索引。`steps` 为正向下、负向上。
    fn scroll_selection(&mut self, steps: i32) {
        match self.phase {
            AppPhase::Locked => {
                let count = crate::screens::locked::ACTION_COUNT as i32;
                self.locked_selected =
                    (self.locked_selected as i32 + steps).clamp(0, count - 1) as usize;
            }
            AppPhase::Welcome => {
                let count = crate::screens::welcome::WELCOME_ACTIONS.len() as i32;
                self.welcome_selected =
                    (self.welcome_selected as i32 + steps).clamp(0, count - 1) as usize;
            }
            AppPhase::Home { .. } => {
                let count = crate::screens::home::shortcut_count() as i32;
                self.selected_shortcut =
                    (self.selected_shortcut as i32 + steps).clamp(0, count - 1) as usize;
            }
            _ => {}
        }
    }

    /// 登录向导的键盘处理。
    fn handle_unlock_key(&mut self, key: KeyEvent, step: UnlockStep) -> Result<bool> {
        match step {
            UnlockStep::SelectAccount {
                accounts,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        commands::core::back(self);
                    }
                    KeyCode::Up if selected > 0 => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down if selected + 1 < accounts.len() => {
                        selected += 1;
                    }
                    KeyCode::Enter => {
                        let account = &accounts[selected];
                        let account_id = account.id.clone();
                        let account_name = account.name.clone();
                        let password_hint = account.password_hint.clone();
                        let (biometric_configured, biometry_type) =
                            self.biometric_status(&account_id);
                        self.phase = AppPhase::UnlockWizard {
                            step: UnlockStep::EnterPassword {
                                account_id,
                                account_name,
                                password_hint,
                                biometric_configured,
                                biometry_type,
                            },
                        };
                        self.password_input.clear();
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::UnlockWizard {
                    step: UnlockStep::SelectAccount { accounts, selected },
                };
                Ok(false)
            }
            UnlockStep::EnterPassword {
                account_id,
                biometric_configured,
                ..
            } => {
                if key.code == KeyCode::Esc {
                    self.password_input.clear();
                    commands::core::back(self);
                    return Ok(false);
                }

                if key.code == KeyCode::Enter {
                    self.submit_password(&account_id)?;
                    return Ok(false);
                }

                if key.code == KeyCode::Char('b') || key.code == KeyCode::Char('B') {
                    if biometric_configured {
                        self.try_biometric_unlock(&account_id)?;
                    } else {
                        self.error_message = Some("当前账户未启用生物识别登录".to_string());
                    }
                    return Ok(false);
                }

                self.password_input.handle_key(&key);
                Ok(false)
            }
        }
    }

    /// 锁定页的键盘处理。
    ///
    /// 命令框非空或斜杠面板打开时，复用普通命令行按键处理（包括面板 ↑/↓ 导航、
    /// Tab 补全、Esc 等）。只有命令框为空且面板关闭时，↑/↓ 才用于选择按钮。
    fn handle_locked_key(&mut self, key: KeyEvent) -> Result<bool> {
        let palette_active = self.command_palette.should_render(&self.command_input);

        if !self.command_input.is_empty() || palette_active {
            return self.handle_command_key(key);
        }

        match key.code {
            KeyCode::Up if self.locked_selected > 0 => {
                self.locked_selected -= 1;
            }
            KeyCode::Down if self.locked_selected + 1 < crate::screens::locked::ACTION_COUNT => {
                self.locked_selected += 1;
            }
            KeyCode::Enter => {
                if let ClickAction::Command(cmd) =
                    crate::screens::locked::ACTIONS[self.locked_selected].action
                {
                    self.command_input.set_value(cmd.to_string());
                    return self.execute_command();
                }
            }
            _ => return self.handle_command_key(key),
        }
        Ok(false)
    }

    /// 欢迎页的键盘处理。
    ///
    /// 命令框非空或斜杠面板打开时，复用普通命令行按键处理。只有命令框为空且面板
    /// 关闭时，↑/↓ 才用于选择选项。
    fn handle_welcome_key(&mut self, key: KeyEvent) -> Result<bool> {
        let palette_active = self.command_palette.should_render(&self.command_input);

        if !self.command_input.is_empty() || palette_active {
            return self.handle_command_key(key);
        }

        match key.code {
            KeyCode::Up if self.welcome_selected > 0 => {
                self.welcome_selected -= 1;
            }
            KeyCode::Down
                if self.welcome_selected + 1 < crate::screens::welcome::WELCOME_ACTIONS.len() =>
            {
                self.welcome_selected += 1;
            }
            KeyCode::Enter => {
                return self.execute_welcome_action();
            }
            _ => return self.handle_command_key(key),
        }
        Ok(false)
    }

    /// 执行当前欢迎页选中的动作。
    fn execute_welcome_action(&mut self) -> Result<bool> {
        match crate::screens::welcome::WELCOME_ACTIONS[self.welcome_selected].action {
            ClickAction::Command(cmd) => {
                self.command_input.set_value(cmd.to_string());
                self.execute_command()
            }
            ClickAction::StartOnboarding => {
                self.phase = AppPhase::Onboarding {
                    step: OnboardingStep::EnterName,
                };
                Ok(false)
            }
        }
    }

    /// 创建账户向导的键盘处理。
    fn handle_onboarding_key(&mut self, key: KeyEvent, step: OnboardingStep) -> Result<bool> {
        match step {
            OnboardingStep::EnterName => match key.code {
                KeyCode::Esc => {
                    self.prompt_exit_onboarding();
                    return Ok(false);
                }
                KeyCode::Enter => {
                    let name = self.command_input.clear().trim().to_string();
                    if name.is_empty() {
                        self.error_message = Some("账户名不能为空".to_string());
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::EnterName,
                        };
                    } else {
                        self.error_message = None;
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::EnterPassword { name },
                        };
                    }
                    return Ok(false);
                }
                _ => {
                    self.command_input.handle_key(&key);
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterName,
                    };
                }
            },
            OnboardingStep::EnterPassword { name } => {
                if key.code == KeyCode::Esc {
                    self.password_input.clear();
                    self.command_input.clear();
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterName,
                    };
                    return Ok(false);
                }
                if key.code == KeyCode::Enter {
                    let password = self.password_input.value().clone();
                    self.password_input.clear();
                    if password.len() < 8 {
                        self.error_message = Some("主密码至少需要 8 位".to_string());
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::EnterPassword { name },
                        };
                    } else {
                        self.error_message = None;
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::ConfirmPassword { name, password },
                        };
                    }
                    return Ok(false);
                }
                self.password_input.handle_key(&key);
                self.phase = AppPhase::Onboarding {
                    step: OnboardingStep::EnterPassword { name },
                };
            }
            OnboardingStep::ConfirmPassword { name, password } => {
                if key.code == KeyCode::Esc {
                    self.password_input.clear();
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterPassword { name },
                    };
                    return Ok(false);
                }
                if key.code == KeyCode::Enter {
                    let confirm = self.password_input.value().clone();
                    self.password_input.clear();
                    if confirm.as_str() != password.as_str() {
                        self.error_message = Some("两次输入的密码不一致，请重新设置".to_string());
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::EnterPassword { name },
                        };
                    } else {
                        self.error_message = None;
                        self.phase = AppPhase::Onboarding {
                            step: OnboardingStep::EnterHint { name, password },
                        };
                    }
                    return Ok(false);
                }
                self.password_input.handle_key(&key);
                self.phase = AppPhase::Onboarding {
                    step: OnboardingStep::ConfirmPassword { name, password },
                };
            }
            OnboardingStep::EnterHint { name, password } => match key.code {
                KeyCode::Esc => {
                    let hint = self.command_input.clear().trim().to_string();
                    let hint = if hint.is_empty() { None } else { Some(hint) };
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::Confirm {
                            name,
                            password,
                            hint,
                        },
                    };
                    return Ok(false);
                }
                KeyCode::Enter => {
                    let hint = self.command_input.clear().trim().to_string();
                    let hint = if hint.is_empty() { None } else { Some(hint) };
                    self.error_message = None;
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::Confirm {
                            name,
                            password,
                            hint,
                        },
                    };
                    return Ok(false);
                }
                _ => {
                    self.command_input.handle_key(&key);
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterHint { name, password },
                    };
                }
            },
            OnboardingStep::Confirm {
                name,
                password,
                hint,
            } => {
                if key.code == KeyCode::Esc {
                    self.command_input
                        .set_value(hint.clone().unwrap_or_default());
                    self.phase = AppPhase::Onboarding {
                        step: OnboardingStep::EnterHint { name, password },
                    };
                    return Ok(false);
                }
                if key.code == KeyCode::Enter {
                    self.create_account_and_enter(name, password, hint)?;
                    return Ok(false);
                }
                // 其他按键忽略
                self.phase = AppPhase::Onboarding {
                    step: OnboardingStep::Confirm {
                        name,
                        password,
                        hint,
                    },
                };
            }
        }
        Ok(false)
    }

    fn prompt_exit_onboarding(&mut self) {
        prompt::open(
            self,
            PromptSpec::Confirm {
                message: "退出创建账户？未保存的数据将不会被保留。".to_string(),
                default_yes: false,
            },
            Box::new(|app, result| {
                if let PromptResult::Confirm(true) = result {
                    app.vault_service.lock();
                    app.password_input.clear();
                    app.phase = AppPhase::Quit;
                }
            }),
        );
    }

    fn prompt_exit_cli(&mut self) {
        prompt::open(
            self,
            PromptSpec::Confirm {
                message: "退出 SoloSoul CLI？".to_string(),
                default_yes: false,
            },
            Box::new(|app, result| {
                if let PromptResult::Confirm(true) = result {
                    commands::core::exit(app);
                }
            }),
        );
    }

    fn create_account_and_enter(
        &mut self,
        name: String,
        password: Zeroizing<String>,
        hint: Option<String>,
    ) -> Result<()> {
        let hint_ref = hint.as_deref();
        match self
            .vault_service
            .create_account(&name, password.as_ref(), hint_ref)
        {
            Ok(account) => {
                let account_id = account["id"].as_str().unwrap_or("").to_string();
                if let Some(vault) = self.vault_service.get_vault_store() {
                    let locale = sys_locale::get_locale()
                        .map(|l| {
                            if l.starts_with("zh") {
                                "zh-CN".to_string()
                            } else {
                                l
                            }
                        })
                        .unwrap_or_else(|| "en-US".to_string());
                    if let Err(e) = solosoul_core::template_service::seed_default_templates(
                        &vault,
                        &account_id,
                        &locale,
                    ) {
                        tracing::warn!("导入默认模板失败: {}", e);
                    }
                }
                self.error_message = None;
                self.enter_home(&account_id);
            }
            Err(e) => {
                self.error_message = Some(format!("创建账户失败: {}", e));
                self.phase = AppPhase::Onboarding {
                    step: OnboardingStep::EnterName,
                };
            }
        }
        Ok(())
    }

    /// 提交密码进行解锁。
    fn submit_password(&mut self, account_id: &str) -> Result<()> {
        let password = self.password_input.value().clone();
        self.password_input.clear();

        match self.vault_service.unlock_secure(account_id, &password) {
            Ok(()) => {
                drop(password);
                self.enter_home(account_id);
            }
            Err(e) => {
                drop(password);
                self.error_message = Some(format!("登录失败: {}", e));
            }
        }
        Ok(())
    }

    /// 获取指定账户的生物识别状态。
    fn biometric_status(&self, account_id: &str) -> (bool, Option<String>) {
        use solosoul_core::biometric::BiometricManager;
        let manager = BiometricManager::new(self.vault_service.base_path().to_path_buf());
        let avail = manager.availability(account_id);
        if avail.configured {
            (true, avail.biometry_type)
        } else {
            (false, None)
        }
    }

    /// 尝试使用生物识别解锁 Vault。
    fn try_biometric_unlock(&mut self, account_id: &str) -> Result<()> {
        use solosoul_core::biometric::BiometricManager;
        let manager = BiometricManager::new(self.vault_service.base_path().to_path_buf());
        match manager.unlock(account_id, &self.vault_service, "解锁 SoloSoul Vault") {
            Ok(kind) => {
                self.error_message = None;
                self.enter_home(account_id);
                tracing::info!("biometric unlock succeeded: {}", kind);
            }
            Err(e) => {
                self.error_message = Some(format!("生物识别解锁失败: {}", e));
            }
        }
        Ok(())
    }

    /// 普通命令模式键盘处理。
    fn handle_command_key(&mut self, key: KeyEvent) -> Result<bool> {
        // 全局 Esc：先清 error overlay；若斜杠面板打开则关闭面板；否则清空输入/返回
        if key.code == KeyCode::Esc {
            if self.error_message.take().is_some() {
                return Ok(false);
            }
            if self.command_palette.should_render(&self.command_input) {
                self.command_palette.suppress();
                return Ok(false);
            }
            if !self.command_input.is_empty() {
                self.command_input.clear();
                return Ok(false);
            }
            if matches!(self.phase, AppPhase::Home { .. }) {
                self.prompt_exit_cli();
            } else {
                commands::core::back(self);
            }
            return Ok(false);
        }

        // 全局 Ctrl+L：手动锁定 Vault
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L'))
        {
            if self.vault_service.is_unlocked() {
                commands::auth::lock(self);
            }
            return Ok(false);
        }

        // 命令历史翻阅（仅当命令框为空、面板未激活且不在首页时；首页 ↑/↓ 用于选项导航）
        if matches!(key.code, KeyCode::Up | KeyCode::Down)
            && self.command_input.is_empty()
            && !self.command_palette.should_render(&self.command_input)
            && !matches!(self.phase, AppPhase::Home { .. })
        {
            self.handle_history(key.code);
            return Ok(false);
        }

        // 斜杠命令面板导航键优先处理
        if self.command_palette.should_render(&self.command_input) {
            let candidates = CommandPalette::build_candidates(
                available_commands(&self.phase),
                &self.command_input.value,
            );
            if !candidates.is_empty() {
                match self.command_palette.handle_key(&key, &candidates) {
                    PaletteAction::Close => return Ok(false),
                    PaletteAction::Fill(cmd) => {
                        self.command_input.set_value(cmd.to_string());
                        return Ok(false);
                    }
                    PaletteAction::Execute(cmd) => {
                        self.command_input.set_value(cmd.to_string());
                        return self.execute_command();
                    }
                    PaletteAction::None => {}
                }
            }
        }

        // 命令输入框消费普通字符/编辑键
        if self.command_input.handle_key(&key) {
            self.command_palette.clear_suppress();
            return Ok(false);
        }

        // 首页选项导航（命令框为空时）：↑/↓/Tab/Shift+Tab 循环选择。
        let is_home = matches!(self.phase, AppPhase::Home { .. });

        if is_home
            && self.command_input.is_empty()
            && matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab
            )
        {
            let count = crate::screens::home::shortcut_count();
            match key.code {
                KeyCode::Up | KeyCode::BackTab => {
                    self.selected_shortcut = (self.selected_shortcut + count - 1) % count;
                }
                KeyCode::Down | KeyCode::Tab => {
                    self.selected_shortcut = (self.selected_shortcut + 1) % count;
                }
                _ => {}
            }
            return Ok(false);
        }

        // 全局快捷键
        if key.code == KeyCode::Enter {
            if is_home
                && self.command_input.is_empty()
                && self.selected_shortcut < crate::screens::home::shortcut_count()
            {
                if let Some(cmd) = crate::screens::home::SHORTCUTS[self.selected_shortcut].command {
                    self.command_input.set_value(cmd.to_string());
                }
                return Ok(false);
            }
            return self.execute_command();
        }

        if key.code == KeyCode::Tab {
            self.command_input
                .autocomplete(available_commands(&self.phase));
            return Ok(false);
        }

        Ok(false)
    }

    fn handle_history(&mut self, code: KeyCode) {
        if self.command_history.is_empty() {
            return;
        }
        match code {
            KeyCode::Up => {
                let idx = self
                    .history_index
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(self.command_history.len() - 1);
                self.history_index = Some(idx);
                self.command_input.value = self.command_history[idx].clone();
                self.command_input.move_cursor_end();
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.command_history.len() {
                        self.history_index = Some(idx + 1);
                        self.command_input.value = self.command_history[idx + 1].clone();
                    } else {
                        self.history_index = None;
                        self.command_input.clear();
                    }
                    self.command_input.move_cursor_end();
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) -> Result<bool> {
        let cmd = self.command_input.clear().trim().to_string();
        if cmd.is_empty() {
            return Ok(false);
        }
        self.command_history.push(cmd.clone());
        self.history_index = None;
        self.error_message = None;

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let base = parts.first().copied().unwrap_or("");

        match base {
            "/exit" => {
                commands::core::exit(self);
                return Ok(true);
            }
            "/back" => commands::core::back(self),
            "/account_list" => commands::auth::account_list(self)?,
            "/doctor" => commands::doctor::run(self)?,
            "/unlock" | "/login" => commands::auth::unlock(self)?,
            "/lock" | "/logout" => commands::auth::lock(self),
            "/list" => commands::vault_read::list(self, parts.get(1).copied())?,
            "/open" => commands::vault_read::open(self, parts.get(1).copied())?,
            "/size" | "/status" | "/state" => commands::vault_read::size(self)?,
            "/newpage" => commands::vault_write::newpage(self, parts.get(1).copied())?,
            "/newobject" => commands::vault_write::newobject(self, parts.get(1).copied())?,
            "/edit" => commands::vault_write::edit(self, parts.get(1).copied())?,
            "/delete" => commands::vault_write::delete(self, parts.get(1).copied())?,
            "/trash" | "/bin" => commands::vault_write::trash(self, &parts[1..])?,
            "/restore" => commands::vault_write::restore(self, parts.get(1).copied())?,
            "/purge" => commands::vault_write::purge(self, parts.get(1).copied())?,
            "/search" => {
                let rest = cmd
                    .strip_prefix("/search")
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty());
                commands::search::search(self, rest)?
            }
            "/history" => commands::history::history(self, parts.get(1).copied())?,
            "/rollback" => {
                commands::history::rollback(self, parts.get(1).copied(), parts.get(2).copied())?
            }
            "/operation_log" => commands::log::operation_log(self, parts.get(1).copied())?,
            "/export_log" => commands::log::export_log(self, parts.get(1).copied())?,
            "/about" | "/version" => commands::system::about(self)?,
            "/help" => commands::system::help(self, parts.get(1).copied())?,
            "/attach" => commands::attachment::handle(self, &parts[1..])?,
            "/backup" => commands::backup::handle(self, &parts[1..])?,
            "/export" => commands::export_import::handle(self, &parts)?,
            "/import" => commands::export_import::handle(self, &parts)?,
            "/language" | "/theme" | "/setting" | "/debug_log" => {
                commands::settings::handle(self, &parts)?
            }
            "/security" => commands::security::handle(self, &parts)?,
            "/profile" => commands::profile::handle(self, &parts)?,
            "/template" => commands::template::handle(self, &parts)?,
            "/cancel" => self.cancel_wizard(),
            "/save" => self.save_wizard(),
            _ => {
                self.error_message = Some(format!("未知命令: {}", cmd));
            }
        }
        Ok(false)
    }

    /// 取消当前向导，返回首页或上一屏。
    fn cancel_wizard(&mut self) {
        match &self.phase {
            AppPhase::NewObjectWizard { .. } | AppPhase::EditObjectWizard { .. } => {
                if let Some(prev) = self.previous_phase.clone() {
                    self.phase = prev;
                } else if let Some(account_id) = self.vault_service.get_current_account() {
                    self.enter_home(account_id);
                }
            }
            _ => {
                self.error_message = Some("当前不在向导中".to_string());
            }
        }
    }

    /// 保存当前向导。
    fn save_wizard(&mut self) {
        match &self.phase.clone() {
            AppPhase::NewObjectWizard {
                step:
                    NewObjectStep::FillFields {
                        page_id,
                        page_name,
                        template,
                        name,
                        fields,
                        ..
                    },
            } => {
                if let Err(e) = commands::vault_write::save_new_object(
                    self,
                    page_id.clone(),
                    page_name.clone(),
                    template.clone(),
                    name.clone(),
                    fields.clone(),
                ) {
                    self.error_message = Some(format!("保存失败: {}", e));
                }
            }
            AppPhase::EditObjectWizard {
                object_id: _,
                step: EditObjectStep::Overview { object, .. },
            } => {
                if let Err(e) = commands::vault_write::save_edited_object(self, object.clone()) {
                    self.error_message = Some(format!("保存失败: {}", e));
                } else {
                    self.phase = AppPhase::ObjectDetail {
                        object: object.clone(),
                    };
                }
            }
            _ => {
                self.error_message = Some("当前没有可保存的更改".to_string());
            }
        }
    }

    /// 创建对象向导的键盘处理。
    fn handle_new_object_key(&mut self, key: KeyEvent, step: NewObjectStep) -> Result<bool> {
        match step {
            NewObjectStep::SelectPage {
                pages,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.cancel_wizard();
                    }
                    KeyCode::Up if selected > 0 => selected -= 1,
                    KeyCode::Down if selected + 1 < pages.len() => selected += 1,
                    KeyCode::Enter => {
                        let page = &pages[selected];
                        commands::vault_write::start_select_template(
                            self,
                            page.id.clone(),
                            page.name.clone(),
                        )?;
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::NewObjectWizard {
                    step: NewObjectStep::SelectPage { pages, selected },
                };
            }
            NewObjectStep::SelectTemplate {
                page_id,
                page_name,
                templates,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.cancel_wizard();
                    }
                    KeyCode::Up if selected > 0 => selected -= 1,
                    KeyCode::Down if selected < templates.len() => selected += 1,
                    KeyCode::Enter => {
                        let template = if selected == 0 {
                            None
                        } else {
                            templates.get(selected - 1).cloned()
                        };
                        commands::vault_write::start_fill_fields(
                            self, page_id, page_name, template,
                        )?;
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::NewObjectWizard {
                    step: NewObjectStep::SelectTemplate {
                        page_id,
                        page_name,
                        templates,
                        selected,
                    },
                };
            }
            NewObjectStep::FillFields {
                page_id,
                page_name,
                template,
                name,
                fields,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.cancel_wizard();
                        return Ok(false);
                    }
                    KeyCode::Char('s') => {
                        self.save_wizard();
                        return Ok(false);
                    }
                    KeyCode::Char('n') => {
                        let initial = name.clone();
                        prompt::open(
                            self,
                            PromptSpec::Text {
                                label: "对象名称".to_string(),
                                initial,
                                mask: false,
                                allow_toggle_mask: false,
                            },
                            Box::new(move |app, result| {
                                if let PromptResult::Text(new_name) = result {
                                    app.phase = AppPhase::NewObjectWizard {
                                        step: NewObjectStep::FillFields {
                                            page_id: page_id.clone(),
                                            page_name: page_name.clone(),
                                            template: template.clone(),
                                            name: new_name,
                                            fields: fields.clone(),
                                            selected,
                                        },
                                    };
                                }
                            }),
                        );
                        return Ok(false);
                    }
                    KeyCode::Up if selected > 0 => selected -= 1,
                    KeyCode::Down if selected + 1 < fields.len() => selected += 1,
                    KeyCode::Enter => {
                        let field = fields[selected].clone();
                        self.start_field_edit(field);
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::NewObjectWizard {
                    step: NewObjectStep::FillFields {
                        page_id,
                        page_name,
                        template,
                        name,
                        fields,
                        selected,
                    },
                };
            }
        }
        Ok(false)
    }

    /// 编辑对象向导的键盘处理。
    fn handle_edit_object_key(
        &mut self,
        key: KeyEvent,
        object_id: String,
        step: EditObjectStep,
    ) -> Result<bool> {
        match step {
            EditObjectStep::Overview {
                mut object,
                fields,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.cancel_wizard();
                        return Ok(false);
                    }
                    KeyCode::Char('s') => {
                        self.save_wizard();
                        return Ok(false);
                    }
                    KeyCode::Up if selected > 0 => selected -= 1,
                    KeyCode::Down if selected + 1 < fields.len() => selected += 1,
                    KeyCode::Char('n') => {
                        let initial = object.name.clone();
                        prompt::open(
                            self,
                            PromptSpec::Text {
                                label: "对象名称".to_string(),
                                initial,
                                mask: false,
                                allow_toggle_mask: false,
                            },
                            Box::new(move |app, result| {
                                if let PromptResult::Text(new_name) = result {
                                    object.name = new_name;
                                    app.phase = AppPhase::EditObjectWizard {
                                        object_id: object.id.clone(),
                                        step: EditObjectStep::Overview {
                                            object: object.clone(),
                                            fields: field_editor::EditableField::from_properties_and_template(
                                                &object.properties,
                                                None,
                                            ),
                                            selected: 0,
                                        },
                                    };
                                }
                            }),
                        );
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        let field = fields[selected].clone();
                        self.start_field_edit(field);
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::EditObjectWizard {
                    object_id,
                    step: EditObjectStep::Overview {
                        object,
                        fields,
                        selected,
                    },
                };
            }
        }
        Ok(false)
    }

    /// 搜索结果页的键盘处理。
    fn handle_search_results_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::SearchResults {
            items,
            selected,
            query,
            truncated,
            total_scanned,
        } = &self.phase
        {
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down if selected + 1 < items.len() => selected += 1,
                KeyCode::Enter => {
                    self.phase = AppPhase::SearchResults {
                        query: query.clone(),
                        items: items.clone(),
                        selected,
                        truncated: *truncated,
                        total_scanned: *total_scanned,
                    };
                    commands::search::open_selected(self)?;
                    return Ok(false);
                }
                _ => {}
            }
            self.phase = AppPhase::SearchResults {
                query: query.clone(),
                items: items.clone(),
                selected,
                truncated: *truncated,
                total_scanned: *total_scanned,
            };
        }
        Ok(false)
    }

    /// 历史快照页的键盘处理。
    fn handle_history_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::HistoryList {
            object_id,
            snapshots,
            selected,
        } = &self.phase
        {
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down if selected + 1 < snapshots.len() => selected += 1,
                _ => {}
            }
            self.phase = AppPhase::HistoryList {
                object_id: object_id.clone(),
                snapshots: snapshots.clone(),
                selected,
            };
        }
        Ok(false)
    }

    /// 附件列表页的键盘处理。
    fn handle_attachment_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::AttachmentList {
            object_id,
            items,
            show_deleted,
            selected,
        } = &self.phase
        {
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down if selected + 1 < items.len() => selected += 1,
                _ => {}
            }
            self.phase = AppPhase::AttachmentList {
                object_id: object_id.clone(),
                items: items.clone(),
                show_deleted: *show_deleted,
                selected,
            };
        }
        Ok(false)
    }

    /// 备份列表页的键盘处理。
    fn handle_backup_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::BackupList { items, selected } = &self.phase {
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down if selected + 1 < items.len() => selected += 1,
                _ => {}
            }
            self.phase = AppPhase::BackupList {
                items: items.clone(),
                selected,
            };
        }
        Ok(false)
    }

    /// Profile 页的键盘处理。
    fn handle_profile_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::Profile {
            profile,
            data,
            selected,
        } = &self.phase
        {
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down => selected += 1,
                _ => {}
            }
            self.phase = AppPhase::Profile {
                profile: profile.clone(),
                data: data.clone(),
                selected,
            };
        }
        Ok(false)
    }

    /// 模板列表页的键盘处理。
    fn handle_template_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let AppPhase::TemplateList {
            user_templates,
            system_templates,
            selected,
        } = &self.phase
        {
            let total = user_templates.len() + system_templates.len();
            let mut selected = *selected;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    commands::core::back(self);
                    return Ok(false);
                }
                KeyCode::Up if selected > 0 => selected -= 1,
                KeyCode::Down if selected + 1 < total => selected += 1,
                KeyCode::Enter if selected < total => {
                    let id = if selected < user_templates.len() {
                        user_templates[selected].id.clone()
                    } else {
                        system_templates[selected - user_templates.len()]
                            .key
                            .clone()
                    };
                    commands::template::handle(self, &["/template", "show", &id])?;
                    return Ok(false);
                }
                KeyCode::Char('d') if selected < user_templates.len() => {
                    let id = user_templates[selected].id.clone();
                    commands::template::handle(self, &["/template", "delete", &id])?;
                    return Ok(false);
                }
                _ => {}
            }
            self.phase = AppPhase::TemplateList {
                user_templates: user_templates.clone(),
                system_templates: system_templates.clone(),
                selected,
            };
        }
        Ok(false)
    }

    /// 模板详情页的键盘处理。
    fn handle_template_detail_key(&mut self, key: KeyEvent) -> Result<bool> {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            commands::core::back(self);
        }
        Ok(false)
    }

    /// 回收站列表页的键盘处理。
    fn handle_trash_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        let (items, mut selected, mut selected_ids, filter) = if let AppPhase::TrashList {
            items,
            selected,
            selected_ids,
            filter,
        } = &self.phase
        {
            (
                items.clone(),
                *selected,
                selected_ids.clone(),
                filter.clone(),
            )
        } else {
            return Ok(false);
        };

        let mut need_refresh = false;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                commands::core::back(self);
                return Ok(false);
            }
            KeyCode::Up if selected > 0 => selected -= 1,
            KeyCode::Down if selected + 1 < items.len() => selected += 1,
            KeyCode::Char(' ') if selected < items.len() => {
                let id = items[selected].id.clone();
                if let Some(pos) = selected_ids.iter().position(|x| x == &id) {
                    selected_ids.remove(pos);
                } else {
                    selected_ids.push(id);
                }
            }
            KeyCode::Char('r') => {
                let ids = if selected_ids.is_empty() && selected < items.len() {
                    vec![items[selected].id.clone()]
                } else {
                    selected_ids.clone()
                };
                if !ids.is_empty() {
                    commands::vault_write::batch_restore(self, &ids)?;
                    selected_ids.clear();
                    need_refresh = true;
                }
            }
            KeyCode::Char('p') => {
                let ids = if selected_ids.is_empty() && selected < items.len() {
                    vec![items[selected].id.clone()]
                } else {
                    selected_ids.clone()
                };
                if !ids.is_empty() {
                    commands::vault_write::batch_purge(self, &ids)?;
                    selected_ids.clear();
                    need_refresh = true;
                }
            }
            _ => {}
        }

        if need_refresh {
            commands::vault_write::apply_trash_filter(self, filter.clone())?;
            return Ok(false);
        }

        // 限制 selected 在有效范围内
        if !items.is_empty() && selected >= items.len() {
            selected = items.len() - 1;
        }
        self.phase = AppPhase::TrashList {
            items,
            selected,
            selected_ids,
            filter,
        };
        Ok(false)
    }

    /// 渲染一帧。
    pub fn render(&mut self, frame: &mut Frame) {
        self.clickable_regions.clear();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // 顶部状态栏
        let status_bar = crate::widgets::status_bar::render(self);
        frame.render_widget(status_bar, layout[0]);

        // 中间内容区
        match &self.phase {
            AppPhase::Welcome => crate::screens::welcome::render(
                frame,
                layout[1],
                &mut self.clickable_regions,
                self.mouse_pos,
                self.sheen_offset,
                self.welcome_selected,
            ),
            AppPhase::Locked => crate::screens::locked::render(
                frame,
                layout[1],
                &mut self.clickable_regions,
                self.mouse_pos,
                self.sheen_offset,
                self.locked_selected,
            ),
            AppPhase::AccountList { accounts } => {
                crate::screens::account_list::render(frame, layout[1], accounts)
            }
            AppPhase::UnlockWizard { step } => {
                crate::screens::unlock::render(frame, layout[1], step, self.sheen_offset)
            }
            AppPhase::Home { account_id } => crate::screens::home::render(
                frame,
                layout[1],
                &self.account_name,
                account_id,
                &mut self.clickable_regions,
                self.selected_shortcut,
                self.mouse_pos,
                self.sheen_offset,
            ),
            AppPhase::ObjectList { items, title } => {
                crate::screens::object_list::render(frame, layout[1], title, items)
            }
            AppPhase::ObjectDetail { object } => {
                crate::screens::object_detail::render(frame, layout[1], object)
            }
            AppPhase::Size { report } => crate::screens::size::render(frame, layout[1], report),
            AppPhase::Doctor { report } => crate::screens::doctor::render(frame, layout[1], report),
            AppPhase::NewObjectWizard { step } => {
                crate::screens::new_object::render(frame, layout[1], step)
            }
            AppPhase::EditObjectWizard { object_id, step } => {
                crate::screens::edit_object::render(frame, layout[1], object_id, step)
            }
            AppPhase::TrashList {
                items,
                selected,
                selected_ids,
                ..
            } => {
                crate::screens::trash_list::render(frame, layout[1], items, *selected, selected_ids)
            }
            AppPhase::Onboarding { step } => {
                crate::screens::onboarding::render(frame, layout[1], self, step)
            }
            AppPhase::SearchResults {
                query,
                items,
                selected,
                truncated,
                total_scanned,
            } => crate::screens::search_results::render(
                frame,
                layout[1],
                query,
                items,
                *selected,
                *truncated,
                *total_scanned,
            ),
            AppPhase::HistoryList {
                object_id,
                snapshots,
                selected,
            } => crate::screens::history_list::render(
                frame, layout[1], object_id, snapshots, *selected,
            ),
            AppPhase::OperationLog {
                entries, selected, ..
            } => crate::screens::operation_log::render(frame, layout[1], entries, *selected),
            AppPhase::About { info } => crate::screens::about::render(frame, layout[1], info),
            AppPhase::Help { topic } => crate::screens::help::render(frame, layout[1], topic),
            AppPhase::AttachmentList {
                object_id,
                items,
                show_deleted,
                selected,
            } => crate::screens::attachment_list::render(
                frame,
                layout[1],
                object_id,
                items,
                *show_deleted,
                *selected,
            ),
            AppPhase::BackupList { items, selected } => {
                crate::screens::backup_list::render(frame, layout[1], items, *selected)
            }
            AppPhase::Profile {
                profile,
                data,
                selected,
            } => crate::screens::profile::render(frame, layout[1], profile, data, *selected),
            AppPhase::TemplateList {
                user_templates,
                system_templates,
                selected,
            } => crate::screens::template_list::render(
                frame,
                layout[1],
                user_templates,
                system_templates,
                *selected,
            ),
            AppPhase::TemplateDetail {
                template_id,
                name,
                source,
                json,
            } => crate::screens::template_detail::render(
                frame,
                layout[1],
                template_id,
                name,
                source,
                json,
            ),
            AppPhase::Quit => {}
        }

        // 底部命令输入框（创建账户向导、模态提示打开时除外；登录密码页改为渲染密码输入框）
        let is_password_page = matches!(
            self.phase,
            AppPhase::UnlockWizard {
                step: UnlockStep::EnterPassword { .. }
            }
        );
        let hide_input = self.prompt.is_some() || matches!(self.phase, AppPhase::Onboarding { .. });
        if !hide_input {
            if is_password_page {
                self.password_input.render(frame, layout[2]);
            } else {
                self.command_input.render(frame, layout[2]);

                if self.command_palette.should_render(&self.command_input) {
                    let candidates = CommandPalette::build_candidates(
                        available_commands(&self.phase),
                        &self.command_input.value,
                    );
                    self.command_palette.render(frame, layout[2], &candidates);
                }
            }
        }

        // 模态提示 overlay
        prompt::render(self, frame);

        // 全局错误 overlay
        if let Some(err) = &self.error_message {
            render_error_overlay(frame, err);
        }
    }
}

/// 根据当前阶段返回可用的命令补全候选。
fn available_commands(phase: &AppPhase) -> &'static [&'static str] {
    match phase {
        AppPhase::Welcome | AppPhase::Locked => {
            &["/account_list", "/doctor", "/exit", "/unlock", "/login"]
        }
        AppPhase::Home { .. } => &[
            "/account_list",
            "/back",
            "/doctor",
            "/exit",
            "/lock",
            "/logout",
            "/list",
            "/open",
            "/size",
            "/status",
            "/state",
            "/search",
            "/history",
            "/rollback",
            "/newpage",
            "/newobject",
            "/edit",
            "/delete",
            "/trash",
            "/bin",
            "/restore",
            "/purge",
            "/operation_log",
            "/export_log",
            "/attach",
            "/backup",
            "/export",
            "/import",
            "/language",
            "/theme",
            "/setting",
            "/security",
            "/debug_log",
            "/about",
            "/version",
            "/help",
        ],
        AppPhase::UnlockWizard { .. } => &["/back"],
        AppPhase::NewObjectWizard { .. } | AppPhase::EditObjectWizard { .. } => {
            // 向导内部仅提供不会丢失未保存数据的命令
            &["/cancel", "/save", "/back"]
        }
        // 其他结果页至少支持 /back 和 /exit
        _ => &["/back", "/exit"],
    }
}

fn render_error_overlay(frame: &mut Frame, message: &str) {
    use ratatui::style::{Style, Stylize};
    use ratatui::text::{Line, Text};
    use ratatui::widgets::{Clear, Paragraph, Wrap};

    let area = frame.area();
    let width = (area.width as f32 * 0.6).min(60.0) as u16;
    let height = 3u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, width, height);

    let text = Text::from(vec![
        Line::from("! 错误").bold(),
        Line::from(message),
        Line::from("按 Esc 关闭").dark_gray(),
    ]);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(Style::default().white().on_red())
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL));

    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    use super::*;

    fn mouse_event(kind: MouseEventKind, column: u16, row: u16) -> crate::events::Event {
        crate::events::Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        })
    }

    fn locked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        vault.lock();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_available_commands_locked() {
        assert_eq!(
            super::available_commands(&AppPhase::Locked),
            &["/account_list", "/doctor", "/exit", "/unlock", "/login"]
        );
    }

    #[test]
    fn test_available_commands_home() {
        assert!(super::available_commands(&AppPhase::Home {
            account_id: "acc".to_string()
        })
        .contains(&"/list"));
    }

    #[test]
    fn test_unlock_success_and_lock() {
        let (mut app, account_id, _dir) = locked_app();
        assert!(matches!(app.phase, AppPhase::Locked));

        // 进入登录向导（单账户直接到密码输入）
        commands::auth::unlock(&mut app).unwrap();
        assert!(matches!(
            app.phase,
            AppPhase::UnlockWizard {
                step: UnlockStep::EnterPassword { .. }
            }
        ));

        // 输入密码
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert_eq!(app.vault_service.get_current_account(), Some(account_id));
        assert_eq!(app.account_name, "Test");

        // 锁定
        commands::auth::lock(&mut app);
        assert!(matches!(app.phase, AppPhase::Locked));
        assert!(!app.vault_service.is_unlocked());
    }

    #[test]
    fn test_home_esc_opens_exit_prompt() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();
        assert!(matches!(app.phase, AppPhase::Home { .. }));

        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Esc)))
            .unwrap();
        assert!(app.prompt.is_some());
    }

    #[test]
    fn test_unlock_wrong_password() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();

        for c in "wrongpass".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        assert!(!app.vault_service.is_unlocked());
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_auto_lock() {
        let (mut app, _id, _dir) = locked_app();
        // 先手动解锁
        commands::auth::unlock(&mut app).unwrap();
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();
        assert!(app.vault_service.is_unlocked());

        // 模拟超时无操作
        app.last_activity = Instant::now() - Duration::from_secs(400);
        app.handle_event(crate::events::Event::Tick).unwrap();

        assert!(!app.vault_service.is_unlocked());
        assert!(matches!(app.phase, AppPhase::Locked));
    }

    #[test]
    fn test_render_home_does_not_panic() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    fn onboarding_app() -> (App, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, dir)
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.handle_event(crate::events::Event::Key(KeyEvent::from(key)))
            .unwrap();
    }

    fn type_string(app: &mut App, s: &str) {
        for c in s.chars() {
            press_key(app, KeyCode::Char(c));
        }
    }

    #[test]
    fn test_onboarding_creates_account() {
        let (mut app, _dir) = onboarding_app();
        assert!(matches!(app.phase, AppPhase::Welcome));

        // 从欢迎页进入创建账户向导
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::EnterName
            }
        ));

        type_string(&mut app, "Alice");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::EnterPassword { .. }
            }
        ));

        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::ConfirmPassword { .. }
            }
        ));

        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::EnterHint { .. }
            }
        ));

        type_string(&mut app, "my favorite color");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::Confirm { .. }
            }
        ));

        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert!(app.vault_service.is_unlocked());
        assert!(app.vault_service.has_any_account());

        // 验证默认模板已导入
        let account_id = app.vault_service.get_current_account().unwrap();
        let vault = app.vault_service.get_vault_store().unwrap();
        assert!(!vault.list_user_templates(&account_id).unwrap().is_empty());
    }

    #[test]
    fn test_home_shortcut_navigation() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert_eq!(app.selected_shortcut, 0);

        // ↓ 前进
        press_key(&mut app, KeyCode::Down);
        assert_eq!(app.selected_shortcut, 1);

        // Tab 继续前进并循环
        for _ in 0..5 {
            press_key(&mut app, KeyCode::Tab);
        }
        assert_eq!(app.selected_shortcut, 0);

        // ↑ 后退
        press_key(&mut app, KeyCode::Up);
        assert_eq!(
            app.selected_shortcut,
            crate::screens::home::shortcut_count() - 1
        );

        // Enter 填入命令
        press_key(&mut app, KeyCode::Enter);
        if let Some(cmd) = crate::screens::home::SHORTCUTS[app.selected_shortcut].command {
            assert_eq!(app.command_input.value, cmd);
        }
    }

    #[test]
    fn test_home_tab_autocomplete_when_input_not_empty() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);

        type_string(&mut app, "/ex");
        press_key(&mut app, KeyCode::Tab);
        assert_eq!(app.command_input.value, "/exit");
        assert_eq!(app.selected_shortcut, 0);
    }

    #[test]
    fn test_slash_palette_navigation_and_execute() {
        use ratatui::backend::TestBackend;

        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(app.vault_service.is_unlocked());

        // 输入 `/lo` 触发斜杠命令面板
        type_string(&mut app, "/lo");
        assert!(app.command_palette.should_render(&app.command_input));

        // 渲染不应 panic
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();

        // 向下选择 /logout，Enter 填入命令，再次 Enter 执行
        press_key(&mut app, KeyCode::Down);
        press_key(&mut app, KeyCode::Enter);
        assert_eq!(app.command_input.value, "/logout");

        press_key(&mut app, KeyCode::Enter);
        assert!(!app.vault_service.is_unlocked());
        assert!(matches!(app.phase, AppPhase::Locked));
    }

    #[test]
    fn test_locked_mouse_wheel_scroll() {
        let (mut app, _id, _dir) = locked_app();
        assert!(matches!(app.phase, AppPhase::Locked));
        assert_eq!(app.locked_selected, 0);

        app.handle_event(mouse_event(MouseEventKind::ScrollDown, 0, 0))
            .unwrap();
        assert_eq!(app.locked_selected, 1);

        app.handle_event(mouse_event(MouseEventKind::ScrollUp, 0, 0))
            .unwrap();
        assert_eq!(app.locked_selected, 0);

        // 滚动不会超出边界
        for _ in 0..10 {
            app.handle_event(mouse_event(MouseEventKind::ScrollDown, 0, 0))
                .unwrap();
        }
        assert_eq!(
            app.locked_selected,
            crate::screens::locked::ACTION_COUNT - 1
        );
    }

    #[test]
    fn test_locked_touch_drag_scroll() {
        let (mut app, _id, _dir) = locked_app();
        assert!(matches!(app.phase, AppPhase::Locked));
        assert_eq!(app.locked_selected, 0);

        // 按下
        app.handle_event(mouse_event(MouseEventKind::Down(MouseButton::Left), 10, 10))
            .unwrap();
        assert_eq!(app.locked_selected, 0);

        // 向下拖拽超过阈值
        app.handle_event(mouse_event(MouseEventKind::Drag(MouseButton::Left), 10, 15))
            .unwrap();
        assert_eq!(app.locked_selected, 1);

        // 释放时不应触发点击（因为是滑动不是点按）
        app.handle_event(mouse_event(MouseEventKind::Up(MouseButton::Left), 10, 15))
            .unwrap();
        assert!(matches!(app.phase, AppPhase::Locked));
    }

    #[test]
    fn test_locked_mouse_click_after_down_up() {
        let (mut app, _id, _dir) = locked_app();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();

        let region = app
            .clickable_regions
            .iter()
            .find(|r| r.action == ClickAction::Command("/unlock"))
            .unwrap()
            .rect;
        let col = region.x + region.width / 2;
        let row = region.y + region.height / 2;

        // 按下时不执行命令
        app.handle_event(mouse_event(
            MouseEventKind::Down(MouseButton::Left),
            col,
            row,
        ))
        .unwrap();
        assert!(matches!(app.phase, AppPhase::Locked));

        // 松开时执行点击动作
        app.handle_event(mouse_event(MouseEventKind::Up(MouseButton::Left), col, row))
            .unwrap();
        assert!(matches!(
            app.phase,
            AppPhase::UnlockWizard {
                step: UnlockStep::EnterPassword { .. }
            }
        ));
    }

    #[test]
    fn test_welcome_key_navigation() {
        let (mut app, _dir) = onboarding_app();
        assert!(matches!(app.phase, AppPhase::Welcome));
        assert_eq!(app.welcome_selected, 0);

        // ↓ 切换选项
        press_key(&mut app, KeyCode::Down);
        assert_eq!(app.welcome_selected, 1);

        // ↑ 切换回第一个
        press_key(&mut app, KeyCode::Up);
        assert_eq!(app.welcome_selected, 0);

        // Enter 执行当前选项（默认开始创建账户）
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(
            app.phase,
            AppPhase::Onboarding {
                step: OnboardingStep::EnterName
            }
        ));
    }

    #[test]
    fn test_welcome_mouse_wheel_scroll() {
        let (mut app, _dir) = onboarding_app();
        assert!(matches!(app.phase, AppPhase::Welcome));
        assert_eq!(app.welcome_selected, 0);

        app.handle_event(mouse_event(MouseEventKind::ScrollDown, 0, 0))
            .unwrap();
        assert_eq!(app.welcome_selected, 1);

        app.handle_event(mouse_event(MouseEventKind::ScrollUp, 0, 0))
            .unwrap();
        assert_eq!(app.welcome_selected, 0);
    }

    #[test]
    fn test_home_mouse_wheel_scroll() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert_eq!(app.selected_shortcut, 0);

        app.handle_event(mouse_event(MouseEventKind::ScrollDown, 0, 0))
            .unwrap();
        assert_eq!(app.selected_shortcut, 1);

        app.handle_event(mouse_event(MouseEventKind::ScrollUp, 0, 0))
            .unwrap();
        assert_eq!(app.selected_shortcut, 0);
    }

    #[test]
    fn test_home_up_down_no_conflict_with_command_input() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        type_string(&mut app, "password123");
        press_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert_eq!(app.selected_shortcut, 0);

        // 命令框非空时，↑/↓ 不用于选项导航
        type_string(&mut app, "/ex");
        press_key(&mut app, KeyCode::Down);
        assert_eq!(app.selected_shortcut, 0);
        press_key(&mut app, KeyCode::Up);
        assert_eq!(app.selected_shortcut, 0);
    }
}
