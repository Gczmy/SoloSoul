//! Fixed categories only; no preference key, value, account ID or error text is retained.
use super::RUNTIME;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const MAX_ATTEMPTS: usize = 256;
static RECORDS: OnceLock<Arc<Mutex<Records>>> = OnceLock::new();

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Theme,
    BackupReminder,
    Other,
}

fn classify<'a>(mut keys: impl Iterator<Item = &'a str>) -> Kind {
    match (keys.next(), keys.next()) {
        (Some("theme"), None) => Kind::Theme,
        (Some("lastBackupReminderAt"), None) => Kind::BackupReminder,
        _ => Kind::Other,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    id: u32,
    kind: Kind,
    account_verified: bool,
    started_at_ms: f64,
    ended_at_ms: Option<f64>,
    outcome: &'static str,
}

struct Records {
    clock: Instant,
    attempts: Vec<Row>,
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

    fn snapshot(&self, run_id: &str, pid: u32) -> Value {
        json!({
            "schemaVersion":1,"scope":"windows-native-preference-writes",
            "runId":run_id,"pid":pid,"clockScope":"process-monotonic",
            "atMs":self.at(),"valid":self.invalid_reasons.is_empty(),
            "invalidReasons":self.invalid_reasons,"maxAttempts":MAX_ATTEMPTS,
            "coverage":"user_data_update_preference","attempts":self.attempts
        })
    }
}

pub struct Attempt {
    records: Arc<Mutex<Records>>,
    index: usize,
    completed: bool,
}

fn begin_in(records: Arc<Mutex<Records>>, kind: Kind, matched: bool) -> Option<Attempt> {
    let mut rows = records.lock().ok()?;
    if !matched {
        rows.invalidate("account-mismatch");
    }
    if rows.attempts.len() >= MAX_ATTEMPTS {
        rows.invalidate("attempt-overflow");
        return None;
    }
    let index = rows.attempts.len();
    let at = rows.at();
    rows.attempts.push(Row {
        id: index as u32 + 1,
        kind,
        account_verified: matched,
        started_at_ms: at,
        ended_at_ms: None,
        outcome: "running",
    });
    drop(rows);
    Some(Attempt {
        records,
        index,
        completed: false,
    })
}

pub fn begin<'a>(account_id: &str, keys: impl Iterator<Item = &'a str>) -> Option<Attempt> {
    let config = RUNTIME.get()?;
    if !config.sdk_journey {
        return None;
    }
    let matched = account_id == format!("acc_rf312_{}", config.object_count);
    begin_in(
        RECORDS
            .get_or_init(|| Arc::new(Mutex::new(Records::new())))
            .clone(),
        classify(keys),
        matched,
    )
}

impl Attempt {
    pub fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if let Ok(mut rows) = self.records.lock() {
            let at = rows.at();
            let row = &mut rows.attempts[self.index];
            row.ended_at_ms = Some(at);
            row.outcome = if self.completed {
                "completed"
            } else {
                "interrupted"
            };
        }
    }
}

pub(super) fn snapshot(run_id: &str) -> Option<Value> {
    let config = RUNTIME.get()?;
    if !config.sdk_journey || config.run_id != run_id {
        return None;
    }
    let rows = RECORDS.get_or_init(|| Arc::new(Mutex::new(Records::new())));
    Some(rows.lock().ok()?.snapshot(run_id, std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_single_known_keys_receive_specific_categories() {
        for (keys, expected) in [
            (vec!["theme"], "theme"),
            (vec!["lastBackupReminderAt"], "backup-reminder"),
            (vec!["theme", "lastBackupReminderAt"], "other"),
            (vec!["lastBackupReminderAt", "theme"], "other"),
            (vec!["unrecognized-secret-key"], "other"),
            (vec![], "other"),
        ] {
            assert_eq!(
                serde_json::to_value(classify(keys.into_iter())).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn interrupted_and_completed_attempts_are_distinct_and_contain_no_payload() {
        let records = Arc::new(Mutex::new(Records::new()));
        let first = begin_in(records.clone(), Kind::Theme, true).unwrap();
        let running = records.lock().unwrap().snapshot("run", 7);
        assert_eq!(running["attempts"][0]["outcome"], "running");
        assert!(running["attempts"][0]["endedAtMs"].is_null());
        drop(first);
        begin_in(records.clone(), Kind::BackupReminder, true)
            .unwrap()
            .complete();
        let result = records.lock().unwrap().snapshot("run", 7);
        assert_eq!(result["attempts"][0]["outcome"], "interrupted");
        assert_eq!(result["attempts"][1]["outcome"], "completed");
        for row in result["attempts"].as_array().unwrap() {
            assert_eq!(row.as_object().unwrap().len(), 6);
            assert!(row["endedAtMs"].as_f64().unwrap() >= row["startedAtMs"].as_f64().unwrap());
        }
    }

    #[test]
    fn foreign_account_and_overflow_invalidate_instead_of_silently_losing_calls() {
        let records = Arc::new(Mutex::new(Records::new()));
        begin_in(records.clone(), Kind::Other, false)
            .unwrap()
            .complete();
        for _ in 1..MAX_ATTEMPTS {
            begin_in(records.clone(), Kind::Theme, true)
                .unwrap()
                .complete();
        }
        assert!(begin_in(records.clone(), Kind::Other, true).is_none());
        assert!(begin_in(records.clone(), Kind::Other, true).is_none());
        let result = records.lock().unwrap().snapshot("run", 7);
        assert_eq!(result["valid"], false);
        assert_eq!(result["attempts"].as_array().unwrap().len(), MAX_ATTEMPTS);
        assert_eq!(
            result["invalidReasons"],
            json!(["account-mismatch", "attempt-overflow"])
        );
    }

    #[test]
    fn concurrent_commands_keep_unique_ids_and_their_own_outcomes() {
        let records = Arc::new(Mutex::new(Records::new()));
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|id| {
                let records = records.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let attempt = begin_in(records, Kind::Theme, true).unwrap();
                    if id % 2 == 0 {
                        attempt.complete();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let result = records.lock().unwrap().snapshot("run", 7);
        let rows = result["attempts"].as_array().unwrap();
        assert_eq!(rows.len(), 16);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row["id"], index + 1);
        }
        assert_eq!(
            rows.iter()
                .filter(|row| row["outcome"] == "completed")
                .count(),
            8
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row["outcome"] == "interrupted")
                .count(),
            8
        );
    }
}
