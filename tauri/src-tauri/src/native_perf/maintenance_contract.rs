//! RF-312：维护失败绑定原始认证阶段；只接受固定类别和数字见证。
use super::{
    auth_contract,
    maintenance_trace::{MAX_ACTIVITIES, MAX_FAILURES},
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|obj| obj.len() == keys.len() && keys.iter().all(|key| obj.contains_key(*key)))
}
fn time(value: &Value, min: f64, max: f64) -> bool {
    value
        .as_f64()
        .is_some_and(|v| v.is_finite() && v >= min && v <= max)
}
pub fn valid(value: &Value, backend: &Value, run_id: &str, pid: u32) -> bool {
    if run_id.len() != 32
        || !run_id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || pid == 0
        || !auth_contract::backend(backend, run_id)
        || !exact(
            value,
            &[
                "schemaVersion",
                "scope",
                "runId",
                "pid",
                "clockScope",
                "atMs",
                "valid",
                "invalidReasons",
                "coverage",
                "maxActivities",
                "registeredActivities",
                "maxFailures",
                "failures",
            ],
        )
        || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-maintenance-admission"
        || value["runId"] != run_id
        || value["pid"].as_u64() != Some(pid as u64)
        || value["clockScope"] != "process-monotonic"
        || value["valid"] != true
        || value["invalidReasons"] != json!([])
        || value["coverage"] != json!(["update-source-preferences", "owned-blocking-worker"])
        || value["maxActivities"].as_u64() != Some(MAX_ACTIVITIES as u64)
        || value["maxFailures"].as_u64() != Some(MAX_FAILURES as u64)
        || value["atMs"].as_f64() != backend["atMs"].as_f64()
    {
        return false;
    }
    let Some(registered) = value["registeredActivities"]
        .as_u64()
        .filter(|v| *v <= MAX_ACTIVITIES as u64)
    else {
        return false;
    };
    let Some(failures) = value["failures"]
        .as_array()
        .filter(|v| v.len() <= MAX_FAILURES)
    else {
        return false;
    };
    let at = backend["atMs"].as_f64().unwrap();
    let attempts = backend["attempts"].as_array().unwrap();
    let mut seen = HashSet::new();
    let mut witnesses = HashMap::new();
    let mut previous: f64 = 0.0;
    for failure in failures {
        if !exact(
            failure,
            &["attemptId", "stageToken", "atMs", "errorClass", "witnesses"],
        ) || !matches!(
            failure["errorClass"].as_str(),
            Some("operations-active" | "directory-busy" | "directory-unavailable" | "unclassified")
        ) {
            return false;
        }
        let Some(id) = failure["attemptId"].as_u64() else {
            return false;
        };
        let Some(token) = failure["stageToken"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
        else {
            return false;
        };
        if !seen.insert((id, token)) {
            return false;
        }
        let Some(attempt) = attempts.iter().find(|r| r["id"].as_u64() == Some(id)) else {
            return false;
        };
        if attempt["kind"] != "login" || attempt["rootVerified"] != true {
            return false;
        }
        let Some(stage) = attempt["stages"].as_array().unwrap().get(token) else {
            return false;
        };
        if stage["name"] != "maintenance-admission" || stage["outcome"] == "completed" {
            return false;
        }
        let start = stage["startedAtMs"].as_f64().unwrap();
        let end = stage["endedAtMs"].as_f64().unwrap_or(at);
        if !time(&failure["atMs"], previous.max(start), end) {
            return false;
        }
        previous = failure["atMs"].as_f64().unwrap();
        let Some(active) = failure["witnesses"]
            .as_array()
            .filter(|r| r.len() <= MAX_ACTIVITIES)
        else {
            return false;
        };
        if failure["errorClass"] != "operations-active" && !active.is_empty() {
            return false;
        }
        let mut prior_id = 0;
        for witness in active {
            if !exact(witness, &["id", "kind", "startedAtMs"])
                || !matches!(
                    witness["kind"].as_str(),
                    Some("update-source-preferences" | "owned-blocking-worker")
                )
                || !time(&witness["startedAtMs"], 0.0, start)
            {
                return false;
            }
            let Some(witness_id) = witness["id"]
                .as_u64()
                .filter(|id| *id > prior_id && *id <= registered)
            else {
                return false;
            };
            prior_id = witness_id;
            if let Some(prior) = witnesses.insert(witness_id, witness) {
                if prior != witness {
                    return false;
                }
            }
        }
    }
    // 完成的准入不需要失败；已中断的准入必须有唯一错误，不能漏掉真实失败。
    for attempt in attempts {
        for (token, stage) in attempt["stages"].as_array().unwrap().iter().enumerate() {
            if stage["name"] == "maintenance-admission"
                && stage["outcome"] == "interrupted"
                && !seen.contains(&(attempt["id"].as_u64().unwrap(), token))
            {
                return false;
            }
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    const RUN: &str = "0123456789abcdef0123456789abcdef";
    fn fixture() -> (Value, Value) {
        let backend = json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":RUN,"clockScope":"process-monotonic","atMs":10,"valid":true,"invalidReasons":[],"maxAttempts":64,"maxStages":128,"attempts":[{"id":1,"kind":"login","rootVerified":true,"startedAtMs":2,"endedAtMs":8,"outcome":"interrupted","stages":[{"name":"root-read","startedAtMs":2,"endedAtMs":3,"outcome":"completed"},{"name":"maintenance-admission","startedAtMs":4,"endedAtMs":6,"outcome":"interrupted"}]}]});
        let value = json!({"schemaVersion":1,"scope":"windows-native-maintenance-admission","runId":RUN,"pid":123,"clockScope":"process-monotonic","atMs":10,"valid":true,"invalidReasons":[],"coverage":["update-source-preferences","owned-blocking-worker"],"maxActivities":256,"registeredActivities":2,"maxFailures":64,"failures":[{"attemptId":1,"stageToken":1,"atMs":5,"errorClass":"operations-active","witnesses":[{"id":1,"kind":"update-source-preferences","startedAtMs":1}]}]});
        (value, backend)
    }
    #[test]
    fn fixed_error_witness_is_bound_to_real_interrupted_stage_without_private_fields() {
        let (value, backend) = fixture();
        assert!(valid(&value, &backend, RUN, 123));
        for (pointer, bad) in [
            ("/pid", json!(456)),
            ("/runId", json!("f".repeat(32))),
            ("/atMs", json!(11)),
            ("/valid", json!(false)),
            ("/clockScope", json!("frontend")),
            ("/registeredActivities", json!(257)),
            ("/registeredActivities", json!(0)),
            ("/maxFailures", json!(65)),
            ("/maxActivities", json!(257)),
            ("/coverage", json!(["all-workers"])),
            ("/invalidReasons", json!(["private-error"])),
            ("/failures/0/attemptId", json!(2)),
            ("/failures/0/stageToken", json!(0)),
            ("/failures/0/stageToken", json!(-1)),
            ("/failures/0/atMs", json!(3.5)),
            ("/failures/0/atMs", json!(6.5)),
            ("/failures/0/errorClass", json!("private-error")),
            ("/failures/0/witnesses/0/id", json!(0)),
            ("/failures/0/witnesses/0/kind", json!("private-path")),
            ("/failures/0/witnesses/0/startedAtMs", json!(4.5)),
            ("/failures/0/witnesses/0/startedAtMs", json!(-1)),
        ] {
            let mut bad_value = value.clone();
            *bad_value.pointer_mut(pointer).unwrap() = bad;
            assert!(!valid(&bad_value, &backend, RUN, 123), "{pointer}");
        }
        for pointer in ["", "/failures/0", "/failures/0/witnesses/0"] {
            let mut bad = value.clone();
            bad.pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("path".into(), json!("sentinel"));
            assert!(!valid(&bad, &backend, RUN, 123));
        }
        let mut missing = value.clone();
        missing["failures"] = json!([]);
        assert!(!valid(&missing, &backend, RUN, 123));
        let mut dup = value.clone();
        dup["failures"]
            .as_array_mut()
            .unwrap()
            .push(value["failures"][0].clone());
        assert!(!valid(&dup, &backend, RUN, 123));
        let mut dup = value.clone();
        dup["failures"][0]["witnesses"]
            .as_array_mut()
            .unwrap()
            .push(value["failures"][0]["witnesses"][0].clone());
        assert!(!valid(&dup, &backend, RUN, 123));
        for (pointer, bad) in [
            ("/attempts/0/rootVerified", json!(false)),
            ("/attempts/0/kind", json!("accounts-refresh")),
            ("/attempts/0/stages/1/outcome", json!("completed")),
        ] {
            let mut foreign = backend.clone();
            *foreign.pointer_mut(pointer).unwrap() = bad;
            assert!(!valid(&value, &foreign, RUN, 123));
        }
        assert!(!valid(&value, &backend, RUN, 0));
    }
    #[test]
    fn unknown_witnesses_do_not_mean_idle_and_success_needs_no_failure() {
        let (mut value, mut backend) = fixture();
        value["failures"][0]["witnesses"] = json!([]);
        for kind in [
            "directory-unavailable",
            "directory-busy",
            "unclassified",
            "operations-active",
        ] {
            value["failures"][0]["errorClass"] = json!(kind);
            assert!(valid(&value, &backend, RUN, 123));
        }
        value["failures"] = json!([]);
        backend["attempts"][0]["stages"][1]["outcome"] = json!("completed");
        backend["attempts"][0]["outcome"] = json!("completed");
        assert!(valid(&value, &backend, RUN, 123));
        backend["attempts"][0]["outcome"] = json!("running");
        backend["attempts"][0]["endedAtMs"] = Value::Null;
        backend["attempts"][0]["stages"][1]["outcome"] = json!("running");
        backend["attempts"][0]["stages"][1]["endedAtMs"] = Value::Null;
        assert!(valid(&value, &backend, RUN, 123));
    }
    #[test]
    fn one_witness_must_keep_same_metadata_across_failures() {
        let (mut value, mut backend) = fixture();
        let mut attempt = backend["attempts"][0].clone();
        attempt["id"] = json!(2);
        backend["attempts"].as_array_mut().unwrap().push(attempt);
        let mut failure = value["failures"][0].clone();
        failure["attemptId"] = json!(2);
        value["failures"].as_array_mut().unwrap().push(failure);
        assert!(valid(&value, &backend, RUN, 123));
        value["failures"][1]["witnesses"][0]["kind"] = json!("owned-blocking-worker");
        assert!(!valid(&value, &backend, RUN, 123));
        value["failures"][1]["witnesses"][0]["kind"] = json!("update-source-preferences");
        value["failures"][1]["witnesses"][0]["startedAtMs"] = json!(0.5);
        assert!(!valid(&value, &backend, RUN, 123));
    }
}
