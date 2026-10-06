//! RF-312：只存固定认证阶段和单调时钟；只对已配置的隔离运行生效。
use super::RUNTIME;
use serde::Serialize;
use serde_json::{json, Value};
use solosoul_core::native_perf_unlock::{UnlockObserver, UnlockStage};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const MAX_ATTEMPTS: usize = 64;
const MAX_STAGES: usize = 128;
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
struct Records {
    clock: Instant,
    attempts: Vec<AttemptRecord>,
    invalid_reasons: Vec<&'static str>,
}
impl Records {
    fn new() -> Self {
        Self {
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
}
impl Span {
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
pub fn snapshot(run_id: &str) -> Value {
    let records = RECORDS.get_or_init(|| Arc::new(Mutex::new(Records::new())));
    match records.lock() {
        Ok(rows) => json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":run_id,
            "clockScope":"process-monotonic","atMs":rows.at(),"valid":rows.invalid_reasons.is_empty() && rows.attempts.iter().all(|row|row.root_verified),
            "invalidReasons":rows.invalid_reasons,"maxAttempts":MAX_ATTEMPTS,"maxStages":MAX_STAGES,"attempts":rows.attempts}),
        Err(_) => json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":run_id,
            "clockScope":"process-monotonic","atMs":0.0,"valid":false,"invalidReasons":["trace-lock-poisoned"],"maxAttempts":MAX_ATTEMPTS,"maxStages":MAX_STAGES,"attempts":[]}),
    }
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
}
