//! RF-312：只存固定认证阶段和单调时钟；只对已配置的隔离运行生效。
use super::RUNTIME;
use serde::Serialize;
use serde_json::{json, Value};
use solosoul_core::native_perf_unlock::{
    PermissionOutcome, PermissionTarget, UnlockObserver, UnlockStage,
};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const MAX_ATTEMPTS: usize = 64;
const MAX_STAGES: usize = 128;
pub(super) const MAX_PERMISSION_COMMANDS: usize = 64;
static RECORDS: OnceLock<Arc<Mutex<Records>>> = OnceLock::new();
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StageRecord {
    name: &'static str,
    started_at_ms: f64,
    ended_at_ms: Option<f64>,
    outcome: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttemptRecord {
    id: u32,
    kind: &'static str,
    root_verified: bool,
    started_at_ms: f64,
    ended_at_ms: Option<f64>,
    outcome: &'static str,
    stages: Vec<StageRecord>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionRecord {
    attempt_id: u32,
    stage_token: u32,
    kind: &'static str,
    at_ms: f64,
    outcome: &'static str,
    exit_code: Option<i32>,
    os_error: Option<i32>,
}
struct Records {
    permission_commands: Vec<PermissionRecord>,
    maintenance: super::maintenance_trace::Records,
    clock: Instant,
    attempts: Vec<AttemptRecord>,
    invalid_reasons: Vec<&'static str>,
}
impl Records {
    fn new() -> Self {
        Self {
            permission_commands: Vec::new(),
            maintenance: super::maintenance_trace::Records::default(),
            clock: Instant::now(),
            attempts: Vec::new(),
            invalid_reasons: Vec::new(),
        }
    }
    fn at(&self) -> f64 {
        self.clock.elapsed().as_secs_f64() * 1000.0
    }
    fn invalidate(&mut self, reason: &'static str) {
        if !self.invalid_reasons.contains(&reason) {
            self.invalid_reasons.push(reason);
        }
    }
}
#[derive(Clone)]
pub struct Handle {
    records: Arc<Mutex<Records>>,
    index: usize,
}
pub struct Attempt {
    handle: Handle,
    completed: bool,
}
pub struct Span {
    handle: Handle,
    token: u32,
    completed: bool,
}
pub fn begin_login(account_id: &str) -> Option<Attempt> {
    let config = RUNTIME.get()?;
    if account_id != format!("acc_rf312_{}", config.object_count) {
        return None;
    }
    begin("login")
}
pub fn begin_accounts() -> Option<Attempt> {
    begin("accounts-refresh")
}
fn begin(kind: &'static str) -> Option<Attempt> {
    if !RUNTIME.get()?.sdk_journey {
        return None;
    }
    let records = RECORDS
        .get_or_init(|| Arc::new(Mutex::new(Records::new())))
        .clone();
    let mut rows = records.lock().ok()?;
    if rows.attempts.len() >= MAX_ATTEMPTS {
        rows.invalidate("attempt-overflow");
        return None;
    }
    let index = rows.attempts.len();
    let at = rows.at();
    rows.attempts.push(AttemptRecord {
        id: index as u32 + 1,
        kind,
        root_verified: false,
        started_at_ms: at,
        ended_at_ms: None,
        outcome: "running",
        stages: Vec::new(),
    });
    drop(rows);
    Some(Attempt {
        handle: Handle { records, index },
        completed: false,
    })
}
impl Attempt {
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
    pub fn complete(mut self) {
        self.completed = true;
    }
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if let Ok(mut rows) = self.handle.records.lock() {
            let at = rows.at();
            let row = &mut rows.attempts[self.handle.index];
            row.ended_at_ms = Some(at);
            row.outcome = if self.completed {
                "completed"
            } else {
                "interrupted"
            };
        }
    }
}
impl Handle {
    pub fn verify_root(&self, root: &Path) {
        let matches = RUNTIME.get().is_some_and(|config| root == config.vault);
        if let Ok(mut rows) = self.records.lock() {
            rows.attempts[self.index].root_verified = matches;
            if !matches {
                rows.invalidate("native-root-mismatch");
            }
        }
    }
    pub fn span(&self, name: &'static str) -> Option<Span> {
        let token = self.begin_named(name)?;
        Some(Span {
            handle: self.clone(),
            token,
            completed: false,
        })
    }
    fn begin_named(&self, name: &'static str) -> Option<u32> {
        let mut rows = self.records.lock().ok()?;
        let at = rows.at();
        let attempt = &mut rows.attempts[self.index];
        if attempt.stages.len() >= MAX_STAGES {
            rows.invalidate("stage-overflow");
            return None;
        }
        let token = attempt.stages.len() as u32;
        attempt.stages.push(StageRecord {
            name,
            started_at_ms: at,
            ended_at_ms: None,
            outcome: "running",
        });
        Some(token)
    }
    fn end_named(&self, token: u32, completed: bool) {
        if let Ok(mut rows) = self.records.lock() {
            let at = rows.at();
            if let Some(row) = rows.attempts[self.index].stages.get_mut(token as usize) {
                row.ended_at_ms = Some(at);
                row.outcome = if completed {
                    "completed"
                } else {
                    "interrupted"
                };
            } else {
                rows.invalidate("stage-token-invalid");
            }
        }
    }
}
impl UnlockObserver for Handle {
    fn begin(&self, stage: UnlockStage) -> Option<u32> {
        self.begin_named(stage.name())
    }
    fn end(&self, token: u32, completed: bool) {
        self.end_named(token, completed);
    }
    fn permission(&self, target: PermissionTarget, outcome: PermissionOutcome) {
        let Ok(mut rows) = self.records.lock() else {
            return;
        };
        let attempt = &rows.attempts[self.index];
        if !attempt.root_verified {
            rows.invalidate("permission-root-unverified");
            return;
        }
        if attempt.kind != "login" {
            rows.invalidate("permission-attempt-kind-invalid");
            return;
        }
        let matching: Vec<_> = attempt
            .stages
            .iter()
            .enumerate()
            .filter(|(_, stage)| stage.name == target.stage().name() && stage.outcome == "running")
            .map(|(index, _)| index as u32)
            .collect();
        // 同一个 worker 的其他权限操作不属于账户清单两个观测阶段。
        if matching.is_empty() {
            return;
        }
        if matching.len() != 1 {
            rows.invalidate("permission-stage-ambiguous");
            return;
        }
        let attempt_id = attempt.id;
        let stage_token = matching[0];
        if rows.permission_commands.len() >= MAX_PERMISSION_COMMANDS {
            rows.invalidate("permission-command-overflow");
            return;
        }
        if rows
            .permission_commands
            .iter()
            .any(|row| row.attempt_id == attempt_id && row.stage_token == stage_token)
        {
            rows.invalidate("permission-command-duplicate");
            return;
        }
        let (name, exit_code, os_error) = match outcome {
            PermissionOutcome::UsernameUnavailable => ("username-unavailable", None, None),
            PermissionOutcome::InvalidUsername => ("invalid-username", None, None),
            PermissionOutcome::SpawnError { os_error } => ("spawn-error", None, os_error),
            PermissionOutcome::Exited {
                code: Some(0),
                success: true,
            } => ("success", Some(0), None),
            PermissionOutcome::Exited {
                code,
                success: false,
            } if code != Some(0) => ("exit-failure", code, None),
            _ => {
                rows.invalidate("permission-status-inconsistent");
                return;
            }
        };
        let at_ms = rows.at();
        rows.permission_commands.push(PermissionRecord {
            attempt_id,
            stage_token,
            kind: target.name(),
            at_ms,
            outcome: name,
            exit_code,
            os_error,
        });
    }
}
/// 仅采集显式隔离 root 的实际许可，正式账户/其他目录不登记。
pub fn register_activity(
    kind: super::maintenance_trace::ActivityKind,
    guard: &Arc<solosoul_core::import_activity::RootActivityGuard>,
) {
    let Some(config) = RUNTIME.get().filter(|config| config.sdk_journey) else {
        return;
    };
    let records = RECORDS.get_or_init(|| Arc::new(Mutex::new(Records::new())));
    if let Ok(mut rows) = records.lock() {
        let at = rows.at();
        rows.maintenance.register(guard, kind, &config.vault, at);
    }
}
impl Span {
    pub fn maintenance_rejected(&self, error: &str) {
        let Ok(mut rows) = self.handle.records.lock() else {
            return;
        };
        let attempt = &rows.attempts[self.handle.index];
        let stage = &attempt.stages[self.token as usize];
        if !attempt.root_verified
            || attempt.kind != "login"
            || stage.name != "maintenance-admission"
            || stage.outcome != "running"
        {
            rows.maintenance.invalidate("failure-stage-invalid");
            return;
        }
        let (id, start) = (attempt.id, stage.started_at_ms);
        let at = rows.at();
        rows.maintenance.rejected(id, self.token, start, at, error);
    }
    pub fn complete(mut self) {
        self.completed = true;
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        self.handle.end_named(self.token, self.completed);
    }
}
pub fn complete(span: Option<Span>) {
    if let Some(span) = span {
        span.complete();
    }
}
// 三份诊断在同一把锁下捕获，使用同一时刻，避免部分 worker 更新导致错配。
pub fn snapshots(run_id: &str) -> (Value, Value, Value) {
    let records = RECORDS.get_or_init(|| Arc::new(Mutex::new(Records::new())));
    match records.lock() {
        Ok(rows) => snapshots_from(&rows, run_id),
        Err(_) => (
            json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":run_id,
                "clockScope":"process-monotonic","atMs":0.0,"valid":false,"invalidReasons":["trace-lock-poisoned"],"maxAttempts":MAX_ATTEMPTS,"maxStages":MAX_STAGES,"attempts":[]}),
            json!({"schemaVersion":1,"scope":"windows-native-permission-commands","runId":run_id,"pid":std::process::id(),
                "clockScope":"process-monotonic","atMs":0.0,"valid":false,"invalidReasons":["trace-lock-poisoned"],"maxCommands":MAX_PERMISSION_COMMANDS,"commands":[]}),
            super::maintenance_trace::Records::default().snapshot(run_id, 0.0, false),
        ),
    }
}
fn snapshots_from(rows: &Records, run_id: &str) -> (Value, Value, Value) {
    let at = rows.at();
    let valid =
        rows.invalid_reasons.is_empty() && rows.attempts.iter().all(|row| row.root_verified);
    (
        json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":run_id,
            "clockScope":"process-monotonic","atMs":at,"valid":valid,
            "invalidReasons":rows.invalid_reasons,"maxAttempts":MAX_ATTEMPTS,"maxStages":MAX_STAGES,"attempts":rows.attempts}),
        json!({"schemaVersion":1,"scope":"windows-native-permission-commands","runId":run_id,"pid":std::process::id(),
            "clockScope":"process-monotonic","atMs":at,"valid":valid,
            "invalidReasons":rows.invalid_reasons,"maxCommands":MAX_PERMISSION_COMMANDS,"commands":rows.permission_commands}),
        rows.maintenance.snapshot(run_id, at, valid),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trace_records_only_fixed_stages_and_distinguishes_incomplete_worker() {
        let records = Arc::new(Mutex::new(Records::new()));
        records.lock().unwrap().attempts.push(AttemptRecord {
            id: 1,
            kind: "login",
            root_verified: true,
            started_at_ms: 0.0,
            ended_at_ms: None,
            outcome: "running",
            stages: Vec::new(),
        });
        let handle = Handle {
            records: records.clone(),
            index: 0,
        };
        let attempt = Attempt {
            handle: handle.clone(),
            completed: false,
        };
        complete(handle.span("blocking-queue"));
        let span = handle.span("sync-disable");
        drop(span);
        drop(attempt);
        let rows = records.lock().unwrap();
        assert_eq!(rows.attempts[0].outcome, "interrupted");
        assert_eq!(rows.attempts[0].stages[0].outcome, "completed");
        assert_eq!(rows.attempts[0].stages[1].outcome, "interrupted");
        let value = serde_json::to_value(&rows.attempts).unwrap();
        assert!(!value.to_string().contains("password"));
    }
    fn permission_handle(root_verified: bool) -> Handle {
        let records = Arc::new(Mutex::new(Records::new()));
        records.lock().unwrap().attempts.push(AttemptRecord {
            id: 1,
            kind: "login",
            root_verified,
            started_at_ms: 0.0,
            ended_at_ms: None,
            outcome: "running",
            stages: Vec::new(),
        });
        Handle { records, index: 0 }
    }
    #[test]
    fn maintenance_error_binds_current_verified_login_stage_only() {
        let handle = permission_handle(true);
        let span = handle.span("maintenance-admission").unwrap();
        span.maintenance_rejected("IMPORT_OPERATIONS_ACTIVE");
        drop(span);
        let rows = handle.records.lock().unwrap();
        let (backend, _, maintenance) = snapshots_from(&rows, "0123456789abcdef0123456789abcdef");
        assert!(super::super::maintenance_contract::valid(
            &maintenance,
            &backend,
            "0123456789abcdef0123456789abcdef",
            std::process::id()
        ));
        assert_eq!(
            maintenance["failures"][0]["errorClass"],
            "operations-active"
        );
        drop(rows);
        for (verified, name) in [(false, "maintenance-admission"), (true, "sync-disable")] {
            let handle = permission_handle(verified);
            handle
                .span(name)
                .unwrap()
                .maintenance_rejected("IMPORT_OPERATIONS_ACTIVE");
            let rows = handle.records.lock().unwrap();
            let (_, _, maintenance) = snapshots_from(&rows, "public");
            assert_eq!(maintenance["failures"], json!([]));
            assert_eq!(maintenance["valid"], false);
        }
    }
    #[test]
    fn permission_outcomes_bind_active_stage_and_capture_both_snapshots_atomically() {
        let handle = permission_handle(true);
        handle.permission(
            PermissionTarget::File,
            PermissionOutcome::SpawnError { os_error: Some(5) },
        );
        assert!(handle
            .records
            .lock()
            .unwrap()
            .permission_commands
            .is_empty());
        let token = handle
            .begin(UnlockStage::AccountManifestPermission)
            .unwrap();
        handle.permission(
            PermissionTarget::File,
            PermissionOutcome::SpawnError { os_error: Some(5) },
        );
        handle.end(token, false);
        let rows = handle.records.lock().unwrap();
        let (backend, permissions, maintenance) = snapshots_from(&rows, "public-run-id");
        assert_eq!(backend["atMs"], permissions["atMs"]);
        assert_eq!(backend["atMs"], maintenance["atMs"]);
        assert_eq!(
            backend["attempts"][0]["stages"][0]["outcome"],
            "interrupted"
        );
        assert_eq!(permissions["commands"][0]["outcome"], "spawn-error");
        assert_eq!(permissions["commands"][0]["osError"], 5);
        assert!(permissions["commands"][0]["exitCode"].is_null());
        assert_eq!(permissions["commands"][0]["attemptId"], 1);
        assert_eq!(permissions["commands"][0]["stageToken"], token);
        assert_eq!(
            backend["attempts"][0]["stages"][0]
                .as_object()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(permissions["commands"][0].as_object().unwrap().len(), 7);
        assert!(!permissions.to_string().contains("password"));
    }
    #[test]
    fn permission_outcomes_reject_unverified_root_duplicate_and_inconsistent_status() {
        let wrong = permission_handle(false);
        wrong.begin(UnlockStage::AccountManifestPermission).unwrap();
        wrong.permission(PermissionTarget::File, PermissionOutcome::InvalidUsername);
        assert!(wrong.records.lock().unwrap().permission_commands.is_empty());
        assert_eq!(
            wrong.records.lock().unwrap().invalid_reasons,
            ["permission-root-unverified"]
        );
        let handle = permission_handle(true);
        handle
            .begin(UnlockStage::AccountDirectoryPermission)
            .unwrap();
        handle.permission(
            PermissionTarget::Directory,
            PermissionOutcome::Exited {
                code: Some(0),
                success: true,
            },
        );
        handle.permission(
            PermissionTarget::Directory,
            PermissionOutcome::Exited {
                code: Some(0),
                success: true,
            },
        );
        assert_eq!(handle.records.lock().unwrap().permission_commands.len(), 1);
        assert_eq!(
            handle.records.lock().unwrap().invalid_reasons,
            ["permission-command-duplicate"]
        );
        let bad = permission_handle(true);
        bad.begin(UnlockStage::AccountManifestPermission).unwrap();
        bad.permission(
            PermissionTarget::File,
            PermissionOutcome::Exited {
                code: Some(5),
                success: true,
            },
        );
        assert!(bad.records.lock().unwrap().permission_commands.is_empty());
        assert_eq!(
            bad.records.lock().unwrap().invalid_reasons,
            ["permission-status-inconsistent"]
        );
    }
    #[test]
    fn permission_outcomes_reject_ambiguous_stage_and_bounded_overflow() {
        let ambiguous = permission_handle(true);
        ambiguous
            .begin(UnlockStage::AccountManifestPermission)
            .unwrap();
        ambiguous
            .begin(UnlockStage::AccountManifestPermission)
            .unwrap();
        ambiguous.permission(
            PermissionTarget::File,
            PermissionOutcome::UsernameUnavailable,
        );
        assert_eq!(
            ambiguous.records.lock().unwrap().invalid_reasons,
            ["permission-stage-ambiguous"]
        );
        let full = permission_handle(true);
        full.begin(UnlockStage::AccountManifestPermission).unwrap();
        let mut rows = full.records.lock().unwrap();
        for i in 0..MAX_PERMISSION_COMMANDS {
            rows.permission_commands.push(PermissionRecord {
                attempt_id: i as u32 + 2,
                stage_token: 0,
                kind: "file",
                at_ms: 0.0,
                outcome: "invalid-username",
                exit_code: None,
                os_error: None,
            });
        }
        drop(rows);
        full.permission(PermissionTarget::File, PermissionOutcome::InvalidUsername);
        assert_eq!(
            full.records.lock().unwrap().permission_commands.len(),
            MAX_PERMISSION_COMMANDS
        );
        assert_eq!(
            full.records.lock().unwrap().invalid_reasons,
            ["permission-command-overflow"]
        );
    }
}
