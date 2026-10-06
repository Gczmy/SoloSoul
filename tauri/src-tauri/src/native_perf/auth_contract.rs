//! 独立认证诊断的固定字段/阶段门禁，不接受业务payload或任意错误文本。
use serde_json::Value;
const FRONT_STAGES: &[&str] = &[
    "started",
    "login-await-start",
    "login-await-ok",
    "login-await-error",
    "accounts-await-start",
    "accounts-await-ok",
    "accounts-await-error",
    "state-set-start",
    "state-set-done",
    "finished",
    "failed",
];
const BACK_STAGES: &[&str] = &[
    "root-read",
    "maintenance-admission",
    "sync-disable",
    "blocking-queue",
    "worker-vault-read",
    "core-unlock",
    "audit-call",
    "cleanup-dispatch",
    "accounts-list",
    "recovery",
    "precheck",
    "master-config",
    "kdf",
    "verify",
    "account-write-call",
    "account-manifest-serialize",
    "account-directory-create",
    "account-directory-permission",
    "account-manifest-atomic-write",
    "account-manifest-permission",
    "kdf-upgrade",
    "vault-open",
    "session-publish",
    "pin-reset-call",
];
const ACCOUNT_SUBSTEPS: &[&str] = &[
    "account-manifest-serialize",
    "account-directory-create",
    "account-directory-permission",
    "account-manifest-atomic-write",
    "account-manifest-permission",
];
fn account_substeps(stages: &[Value], at: f64) -> bool {
    let children: Vec<_> = stages
        .iter()
        .filter(|s| {
            s["name"]
                .as_str()
                .is_some_and(|n| ACCOUNT_SUBSTEPS.contains(&n))
        })
        .collect();
    if children.is_empty() {
        return true;
    }
    let parents: Vec<_> = stages
        .iter()
        .filter(|s| s["name"] == "account-write-call")
        .collect();
    if parents.len() != 1 || children.len() > ACCOUNT_SUBSTEPS.len() {
        return false;
    }
    let parent = parents[0];
    let start = parent["startedAtMs"].as_f64().unwrap();
    let end = parent["endedAtMs"].as_f64().unwrap_or(at);
    let mut previous_end = Some(start);
    for (i, child) in children.iter().enumerate() {
        let child_start = child["startedAtMs"].as_f64().unwrap();
        if child["name"] != ACCOUNT_SUBSTEPS[i]
            || child_start < start
            || child["endedAtMs"].as_f64().unwrap_or(at) > end
            || previous_end.is_none_or(|prior| prior > child_start)
        {
            return false;
        }
        previous_end = child["endedAtMs"].as_f64();
    }
    true
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
}
fn time(value: &Value, min: f64, max: f64) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && n >= min && n <= max)
}
pub fn frontend(value: &Value, run_id: &str, origin: &Value) -> bool {
    if !exact(
        value,
        &[
            "schemaVersion",
            "scope",
            "runId",
            "timeOriginMs",
            "atMs",
            "valid",
            "invalidReasons",
            "invokeCount",
            "maxReplies",
            "maxFlows",
            "replies",
            "flows",
        ],
    ) || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-auth-observer"
        || value["runId"] != run_id
        || value["timeOriginMs"] != *origin
        || value["valid"] != true
        || value["invalidReasons"] != serde_json::json!([])
        || value["maxReplies"] != 128
        || value["maxFlows"] != 64
        || !time(&value["atMs"], 0.0, f64::MAX)
    {
        return false;
    }
    let Some(total) = value["invokeCount"].as_u64().filter(|n| *n <= 16384) else {
        return false;
    };
    let Some(replies) = value["replies"].as_array().filter(|v| v.len() <= 128) else {
        return false;
    };
    let Some(flows) = value["flows"].as_array().filter(|v| v.len() <= 64) else {
        return false;
    };
    let at = value["atMs"].as_f64().unwrap();
    let mut last = 0;
    for row in replies {
        if !exact(
            row,
            &[
                "invokeSeq",
                "command",
                "startedAtMs",
                "headersAtMs",
                "status",
            ],
        ) || !matches!(
            row["command"].as_str(),
            Some("login" | "vault_list_accounts")
        ) {
            return false;
        }
        let Some(seq) = row["invokeSeq"]
            .as_u64()
            .filter(|n| *n > last && *n <= total)
        else {
            return false;
        };
        last = seq;
        if !time(&row["startedAtMs"], 0.0, at) {
            return false;
        }
        let start = row["startedAtMs"].as_f64().unwrap();
        match row["status"].as_str() {
            Some("pending") if row["headersAtMs"].is_null() => {}
            Some("tauri-ok" | "tauri-error" | "transport-error" | "sync-throw")
                if time(&row["headersAtMs"], start, at) => {}
            _ => return false,
        }
    }
    for (index, flow) in flows.iter().enumerate() {
        if !exact(flow, &["id", "events"]) || flow["id"].as_u64() != Some(index as u64 + 1) {
            return false;
        }
        let Some(events) = flow["events"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 16)
        else {
            return false;
        };
        if events[0]["stage"] != "started" {
            return false;
        }
        let mut previous = 0.0;
        let mut prior = "";
        for event in events {
            if !exact(event, &["stage", "atMs"])
                || !event["stage"]
                    .as_str()
                    .is_some_and(|s| FRONT_STAGES.contains(&s))
                || !time(&event["atMs"], previous, at)
            {
                return false;
            }
            let current = event["stage"].as_str().unwrap();
            if !transition(prior, current) {
                return false;
            }
            prior = current;
            previous = event["atMs"].as_f64().unwrap();
        }
    }
    true
}
pub fn backend(value: &Value, run_id: &str) -> bool {
    if !exact(
        value,
        &[
            "schemaVersion",
            "scope",
            "runId",
            "clockScope",
            "atMs",
            "valid",
            "invalidReasons",
            "maxAttempts",
            "maxStages",
            "attempts",
        ],
    ) || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-auth-backend"
        || value["runId"] != run_id
        || value["clockScope"] != "process-monotonic"
        || value["valid"] != true
        || value["invalidReasons"] != serde_json::json!([])
        || value["maxAttempts"] != 64
        || value["maxStages"] != 128
        || !time(&value["atMs"], 0.0, f64::MAX)
    {
        return false;
    }
    let at = value["atMs"].as_f64().unwrap();
    let Some(attempts) = value["attempts"].as_array().filter(|v| v.len() <= 64) else {
        return false;
    };
    for (index, row) in attempts.iter().enumerate() {
        if !exact(
            row,
            &[
                "id",
                "kind",
                "rootVerified",
                "startedAtMs",
                "endedAtMs",
                "outcome",
                "stages",
            ],
        ) || row["id"].as_u64() != Some(index as u64 + 1)
            || !matches!(row["kind"].as_str(), Some("login" | "accounts-refresh"))
            || row["rootVerified"] != true
            || !time(&row["startedAtMs"], 0.0, at)
        {
            return false;
        }
        if !ending(row, at) {
            return false;
        }
        let Some(stages) = row["stages"].as_array().filter(|v| v.len() <= 128) else {
            return false;
        };
        let end = row["endedAtMs"].as_f64().unwrap_or(at);
        let mut previous = row["startedAtMs"].as_f64().unwrap();
        for stage in stages {
            if !exact(stage, &["name", "startedAtMs", "endedAtMs", "outcome"])
                || !stage["name"]
                    .as_str()
                    .is_some_and(|s| BACK_STAGES.contains(&s))
                || !time(&stage["startedAtMs"], previous, end)
                || !ending(stage, end)
                || (row["outcome"] != "running" && stage["outcome"] == "running")
                || (row["kind"] == "accounts-refresh"
                    && !matches!(
                        stage["name"].as_str(),
                        Some("blocking-queue" | "worker-vault-read" | "accounts-list")
                    ))
                || (row["kind"] == "login" && stage["name"] == "accounts-list")
            {
                return false;
            }
            previous = stage["startedAtMs"].as_f64().unwrap();
        }
        if !account_substeps(stages, at) {
            return false;
        }
    }
    true
}
fn transition(prior: &str, current: &str) -> bool {
    matches!(
        (prior, current),
        ("", "started")
            | ("started", "login-await-start")
            | ("login-await-start", "login-await-ok" | "login-await-error")
            | ("login-await-error", "failed")
            | ("login-await-ok", "accounts-await-start" | "failed")
            | (
                "accounts-await-start",
                "accounts-await-ok" | "accounts-await-error"
            )
            | (
                "accounts-await-ok" | "accounts-await-error",
                "state-set-start" | "failed"
            )
            | ("state-set-start", "state-set-done" | "failed")
            | ("state-set-done", "finished" | "failed")
    )
}
fn ending(row: &Value, at: f64) -> bool {
    match row["outcome"].as_str() {
        Some("running") => row["endedAtMs"].is_null(),
        Some("completed" | "interrupted") => {
            time(&row["endedAtMs"], row["startedAtMs"].as_f64().unwrap(), at)
        }
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn witness() -> Value {
        json!({"schemaVersion":1,"scope":"windows-native-auth-observer","runId":"abc","timeOriginMs":10,"atMs":5,"valid":true,"invalidReasons":[],"invokeCount":1,"maxReplies":128,"maxFlows":64,"replies":[{"invokeSeq":1,"command":"login","startedAtMs":1,"headersAtMs":3,"status":"tauri-ok"}],"flows":[{"id":1,"events":[{"stage":"started","atMs":0},{"stage":"login-await-start","atMs":1},{"stage":"login-await-ok","atMs":4}]}]})
    }
    #[test]
    fn auth_witness_rejects_foreign_clock_future_events_and_private_fields() {
        let good = witness();
        assert!(frontend(&good, "abc", &json!(10)));
        for (pointer, bad) in [
            ("/runId", json!("foreign")),
            ("/replies/0/headersAtMs", json!(6)),
            ("/replies/0/status", json!("private")),
            ("/flows/0/events/0/stage", json!("private")),
            ("/flows/0/id", json!(2)),
        ] {
            let mut changed = good.clone();
            *changed.pointer_mut(pointer).unwrap() = bad;
            assert!(!frontend(&changed, "abc", &json!(10)));
        }
        let mut leaked = good.clone();
        leaked["replies"][0]["payload"] = json!("sentinel");
        assert!(!frontend(&leaked, "abc", &json!(10)));
        assert!(!frontend(&good, "abc", &json!(11)));
    }
    #[test]
    fn backend_rejects_private_data_and_stage_beyond_completed_attempt() {
        let good = json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":"abc","clockScope":"process-monotonic","atMs":10,"valid":true,"invalidReasons":[],"maxAttempts":64,"maxStages":128,"attempts":[{"id":1,"kind":"login","rootVerified":true,"startedAtMs":1,"endedAtMs":8,"outcome":"completed","stages":[{"name":"core-unlock","startedAtMs":2,"endedAtMs":7,"outcome":"completed"},{"name":"kdf","startedAtMs":3,"endedAtMs":6,"outcome":"completed"}]}]});
        assert!(backend(&good, "abc"));
        for (pointer, bad) in [
            ("/attempts/0/stages/0/endedAtMs", json!(9)),
            ("/attempts/0/rootVerified", json!(false)),
            ("/attempts/0/stages/1/startedAtMs", json!(1)),
            ("/attempts/0/stages/1/name", json!("accounts-list")),
        ] {
            let mut changed = good.clone();
            *changed.pointer_mut(pointer).unwrap() = bad;
            assert!(!backend(&changed, "abc"));
        }
        let mut with_children = good.clone();
        with_children["attempts"][0]["stages"].as_array_mut().unwrap().extend([
            json!({"name":"account-write-call","startedAtMs":4,"endedAtMs":7,"outcome":"completed"}),
            json!({"name":"account-manifest-serialize","startedAtMs":5,"endedAtMs":6,"outcome":"completed"}),
        ]);
        assert!(backend(&with_children, "abc"));
        let mut escaped = with_children.clone();
        escaped["attempts"][0]["stages"][2]["endedAtMs"] = json!(5.5);
        assert!(!backend(&escaped, "abc"));
        let mut wrong_order = with_children.clone();
        wrong_order["attempts"][0]["stages"][3]["name"] = json!("account-directory-create");
        assert!(!backend(&wrong_order, "abc"));
        let mut leaked = good.clone();
        leaked["attempts"][0]["password"] = json!("sentinel");
        assert!(!backend(&leaked, "abc"));
    }
}
