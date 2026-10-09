//! RF-312：只观察实际许可的 Weak，不延长 worker/目录准入的生命周期。
use serde::Serialize;
use serde_json::{json, Value};
use solosoul_core::import_activity::RootActivityGuard;
use std::{
    path::Path,
    sync::{Arc, Weak},
};

pub(super) const MAX_ACTIVITIES: usize = 256;
pub(super) const MAX_FAILURES: usize = 64;
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivityKind {
    UpdateSourcePreferences,
    OwnedBlockingWorker,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Witness {
    id: u32,
    kind: ActivityKind,
    started_at_ms: f64,
}
struct Activity {
    witness: Witness,
    guard: Weak<RootActivityGuard>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Failure {
    attempt_id: u32,
    stage_token: u32,
    at_ms: f64,
    error_class: &'static str,
    witnesses: Vec<Witness>,
}
#[derive(Default)]
pub(super) struct Records {
    activities: Vec<Activity>,
    failures: Vec<Failure>,
    invalid_reasons: Vec<&'static str>,
}
impl Records {
    pub fn invalidate(&mut self, reason: &'static str) {
        if !self.invalid_reasons.contains(&reason) {
            self.invalid_reasons.push(reason);
        }
    }
    pub fn register(
        &mut self,
        guard: &Arc<RootActivityGuard>,
        kind: ActivityKind,
        root: &Path,
        at: f64,
    ) {
        if guard.root_owner().root() != root {
            return;
        }
        if self.activities.len() >= MAX_ACTIVITIES {
            self.invalidate("activity-overflow");
            return;
        }
        self.activities.push(Activity {
            witness: Witness {
                id: self.activities.len() as u32 + 1,
                kind,
                started_at_ms: at,
            },
            guard: Arc::downgrade(guard),
        });
    }
    pub fn rejected(&mut self, attempt_id: u32, token: u32, start: f64, at: f64, error: &str) {
        if self.failures.len() >= MAX_FAILURES {
            self.invalidate("failure-overflow");
            return;
        }
        if self
            .failures
            .iter()
            .any(|r| r.attempt_id == attempt_id && r.stage_token == token)
        {
            self.invalidate("failure-duplicate");
            return;
        }
        let error_class = match error {
            "IMPORT_DIRECTORY_UNAVAILABLE" => "directory-unavailable",
            "IMPORT_DIRECTORY_BUSY" => "directory-busy",
            "IMPORT_OPERATIONS_ACTIVE" => "operations-active",
            _ => "unclassified",
        };
        // 登记早于准入开始，错误返回后仍有强引用：该实际许可覆盖整个准入区间。
        // 不 upgrade Weak；未观测到许可不能证明目录空闲，其他入口并未全量覆盖。
        let witnesses = self
            .activities
            .iter()
            .filter(|row| {
                error_class == "operations-active"
                    && row.witness.started_at_ms <= start
                    && row.guard.strong_count() > 0
            })
            .map(|row| row.witness.clone())
            .collect();
        self.failures.push(Failure {
            attempt_id,
            stage_token: token,
            at_ms: at,
            error_class,
            witnesses,
        });
    }
    pub fn snapshot(&self, run_id: &str, at: f64, auth_valid: bool) -> Value {
        json!({"schemaVersion":1,"scope":"windows-native-maintenance-admission","runId":run_id,"pid":std::process::id(),
            "clockScope":"process-monotonic","atMs":at,"valid":auth_valid && self.invalid_reasons.is_empty(),"invalidReasons":self.invalid_reasons,
            "coverage":["update-source-preferences","owned-blocking-worker"],"maxActivities":MAX_ACTIVITIES,"registeredActivities":self.activities.len(),
            "maxFailures":MAX_FAILURES,"failures":self.failures})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solosoul_core::import_activity::{begin_owned_root_activity, begin_owned_root_maintenance};
    use solosoul_vault::root_owner::VaultRootOwner;
    #[test]
    fn weak_witness_covers_real_lease_without_retaining_it_or_owner() {
        let dir = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(dir.path()).unwrap();
        let root = owner.root().to_path_buf();
        let weak_owner = Arc::downgrade(&owner);
        let guard = Arc::new(begin_owned_root_activity(Arc::clone(&owner)).unwrap());
        let mut rows = Records::default();
        rows.register(&guard, ActivityKind::UpdateSourcePreferences, &root, 1.0);
        assert_eq!(Arc::strong_count(&guard), 1);
        let error = begin_owned_root_maintenance(Arc::clone(&owner))
            .err()
            .unwrap();
        rows.rejected(1, 0, 2.0, 3.0, &error);
        let value = rows.snapshot("public", 4.0, true);
        assert_eq!(value["failures"][0]["errorClass"], "operations-active");
        assert_eq!(
            value["failures"][0]["witnesses"][0]["kind"],
            "update-source-preferences"
        );
        assert!(!value.to_string().contains(root.to_str().unwrap()));
        drop(guard);
        assert!(begin_owned_root_maintenance(Arc::clone(&owner)).is_ok());
        rows.rejected(2, 0, 5.0, 6.0, &error);
        assert_eq!(
            rows.snapshot("public", 7.0, true)["failures"][1]["witnesses"],
            json!([])
        );
        drop(owner);
        assert_eq!(weak_owner.strong_count(), 0);
    }
    #[test]
    fn actual_worker_keeps_witness_until_real_exit() {
        let dir = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(dir.path()).unwrap();
        let guard = Arc::new(begin_owned_root_activity(Arc::clone(&owner)).unwrap());
        let mut rows = Records::default();
        rows.register(&guard, ActivityKind::OwnedBlockingWorker, owner.root(), 1.0);
        let (release, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _guard = guard;
            rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        });
        rows.rejected(
            1,
            0,
            2.0,
            3.0,
            &begin_owned_root_maintenance(Arc::clone(&owner))
                .err()
                .unwrap(),
        );
        assert_eq!(
            rows.snapshot("public", 4.0, true)["failures"][0]["witnesses"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(begin_owned_root_maintenance(owner).is_ok());
        assert_eq!(rows.activities[0].guard.strong_count(), 0);
    }
    #[test]
    fn foreign_root_and_late_registration_do_not_claim_cause() {
        let own = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(own.path()).unwrap();
        let foreign = VaultRootOwner::acquire(other.path()).unwrap();
        let guard = Arc::new(begin_owned_root_activity(foreign).unwrap());
        let mut rows = Records::default();
        rows.register(&guard, ActivityKind::OwnedBlockingWorker, owner.root(), 0.0);
        assert!(rows.activities.is_empty());
        let guard = Arc::new(begin_owned_root_activity(Arc::clone(&owner)).unwrap());
        rows.register(&guard, ActivityKind::OwnedBlockingWorker, owner.root(), 3.0);
        rows.rejected(1, 0, 2.0, 4.0, "IMPORT_OPERATIONS_ACTIVE");
        assert!(rows.failures[0].witnesses.is_empty());
        rows.rejected(2, 0, 3.0, 4.0, "private sentinel error /account/password");
        let value = rows.snapshot("public", 5.0, true);
        assert_eq!(value["failures"][1]["errorClass"], "unclassified");
        assert_eq!(value["failures"][1]["witnesses"], json!([]));
        assert!(!value.to_string().contains("sentinel"));
        for (id, text, name) in [
            (3, "IMPORT_DIRECTORY_UNAVAILABLE", "directory-unavailable"),
            (4, "IMPORT_DIRECTORY_BUSY", "directory-busy"),
        ] {
            rows.rejected(id, 0, 3.0, 4.0, text);
            assert_eq!(rows.failures.last().unwrap().error_class, name);
            assert!(rows.failures.last().unwrap().witnesses.is_empty());
        }
    }
    #[test]
    fn diagnostics_stay_bounded_and_invalidate_duplicate_or_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let owner = VaultRootOwner::acquire(dir.path()).unwrap();
        let guard = Arc::new(begin_owned_root_activity(Arc::clone(&owner)).unwrap());
        let mut rows = Records::default();
        for _ in 0..=MAX_ACTIVITIES {
            rows.register(&guard, ActivityKind::OwnedBlockingWorker, owner.root(), 0.0);
        }
        assert_eq!(rows.activities.len(), MAX_ACTIVITIES);
        assert_eq!(rows.invalid_reasons, ["activity-overflow"]);
        assert_eq!(Arc::strong_count(&guard), 1);
        for id in 1..=MAX_FAILURES {
            rows.rejected(id as u32, 0, 1.0, 2.0, "IMPORT_OPERATIONS_ACTIVE");
        }
        rows.rejected(65, 0, 1.0, 2.0, "IMPORT_OPERATIONS_ACTIVE");
        assert_eq!(rows.failures.len(), MAX_FAILURES);
        assert_eq!(
            rows.invalid_reasons,
            ["activity-overflow", "failure-overflow"]
        );
        let mut duplicate = Records::default();
        duplicate.rejected(1, 0, 1.0, 2.0, "IMPORT_OPERATIONS_ACTIVE");
        duplicate.rejected(1, 0, 1.0, 2.0, "IMPORT_OPERATIONS_ACTIVE");
        assert_eq!(duplicate.failures.len(), 1);
        assert_eq!(duplicate.invalid_reasons, ["failure-duplicate"]);
    }
}
