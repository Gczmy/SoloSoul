//! RF-1097：权限诊断独立固定字段门禁；认证协议及其原严格验证保持。
use super::auth_trace::MAX_PERMISSION_COMMANDS;
use serde_json::Value;
use std::collections::HashSet;
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
}
fn integer_or_null(value: &Value) -> bool {
    value.is_null() || value.as_i64().is_some_and(|n| i32::try_from(n).is_ok())
}
pub fn valid(value: &Value, backend: &Value, run_id: &str, pid: u32) -> bool {
    if !super::auth_contract::backend(backend, run_id)
        || run_id.len() != 32
        || !run_id.bytes().all(|c| c.is_ascii_hexdigit())
        || pid == 0
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
                "maxCommands",
                "commands",
            ],
        )
        || value["schemaVersion"] != 1
        || value["scope"] != "windows-native-permission-commands"
        || value["runId"] != run_id
        || value["pid"].as_u64() != Some(pid as u64)
        || value["clockScope"] != "process-monotonic"
        || value["valid"] != true
        || !value["invalidReasons"]
            .as_array()
            .is_some_and(|v| v.is_empty())
        || value["maxCommands"].as_u64() != Some(MAX_PERMISSION_COMMANDS as u64)
        || value["atMs"].as_f64() != backend["atMs"].as_f64()
    {
        return false;
    }
    let Some(at) = value["atMs"]
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0)
    else {
        return false;
    };
    let Some(commands) = value["commands"]
        .as_array()
        .filter(|v| v.len() <= MAX_PERMISSION_COMMANDS)
    else {
        return false;
    };
    let attempts = backend["attempts"].as_array().unwrap();
    let mut seen = HashSet::new();
    let mut previous = 0.0;
    for command in commands {
        if !exact(
            command,
            &[
                "attemptId",
                "stageToken",
                "kind",
                "atMs",
                "outcome",
                "exitCode",
                "osError",
            ],
        ) || !integer_or_null(&command["exitCode"])
            || !integer_or_null(&command["osError"])
        {
            return false;
        }
        let Some(attempt_id) = command["attemptId"].as_u64() else {
            return false;
        };
        let Some(token) = command["stageToken"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
        else {
            return false;
        };
        if !seen.insert((attempt_id, token)) {
            return false;
        }
        let matching: Vec<_> = attempts
            .iter()
            .filter(|row| row["id"].as_u64() == Some(attempt_id))
            .collect();
        if matching.len() != 1
            || matching[0]["kind"] != "login"
            || matching[0]["rootVerified"] != true
        {
            return false;
        }
        let Some(stage) = matching[0]["stages"].as_array().unwrap().get(token) else {
            return false;
        };
        let name = match command["kind"].as_str() {
            Some("directory") => "account-directory-permission",
            Some("file") => "account-manifest-permission",
            _ => return false,
        };
        if stage["name"] != name {
            return false;
        }
        let Some(time) = command["atMs"].as_f64().filter(|n| n.is_finite()) else {
            return false;
        };
        if time < previous
            || time < stage["startedAtMs"].as_f64().unwrap()
            || time > stage["endedAtMs"].as_f64().unwrap_or(at)
        {
            return false;
        }
        previous = time;
        let status = match command["outcome"].as_str() {
            Some("success") => {
                command["exitCode"].as_i64() == Some(0) && command["osError"].is_null()
            }
            Some("exit-failure") => {
                command["exitCode"].as_i64() != Some(0) && command["osError"].is_null()
            }
            Some("spawn-error") => command["exitCode"].is_null(),
            Some("username-unavailable" | "invalid-username") => {
                command["exitCode"].is_null() && command["osError"].is_null()
            }
            _ => false,
        };
        if !status
            || (stage["outcome"] == "completed" && command["outcome"] != "success")
            || (stage["outcome"] == "interrupted" && command["outcome"] == "success")
        {
            return false;
        }
    }
    // 已结束的两类权限阶段必须有唯一结果；运行中尚未返回的命令可以没有结果。
    for attempt in attempts {
        for (token, stage) in attempt["stages"].as_array().unwrap().iter().enumerate() {
            if matches!(
                stage["name"].as_str(),
                Some("account-directory-permission" | "account-manifest-permission")
            ) && stage["outcome"] != "running"
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
    use serde_json::json;
    const RUN: &str = "0123456789abcdef0123456789abcdef";
    fn witness() -> (Value, Value) {
        let backend = json!({"schemaVersion":1,"scope":"windows-native-auth-backend","runId":RUN,"clockScope":"process-monotonic","atMs":10,"valid":true,"invalidReasons":[],"maxAttempts":64,"maxStages":128,"attempts":[{"id":1,"kind":"login","rootVerified":true,"startedAtMs":0,"endedAtMs":9,"outcome":"completed","stages":[
            {"name":"account-write-call","startedAtMs":0.5,"endedAtMs":8,"outcome":"completed"},
            {"name":"account-manifest-serialize","startedAtMs":1,"endedAtMs":1.5,"outcome":"completed"},
            {"name":"account-directory-create","startedAtMs":2,"endedAtMs":2.5,"outcome":"completed"},
            {"name":"account-directory-permission","startedAtMs":3,"endedAtMs":3.5,"outcome":"completed"},
            {"name":"account-manifest-atomic-write","startedAtMs":4,"endedAtMs":4.5,"outcome":"completed"},
            {"name":"account-manifest-permission","startedAtMs":5,"endedAtMs":5.5,"outcome":"interrupted"}]}]});
        let value = json!({"schemaVersion":1,"scope":"windows-native-permission-commands","runId":RUN,"pid":1234,"clockScope":"process-monotonic","atMs":10,"valid":true,"invalidReasons":[],"maxCommands":64,"commands":[
            {"attemptId":1,"stageToken":3,"kind":"directory","atMs":3.25,"outcome":"success","exitCode":0,"osError":null},
            {"attemptId":1,"stageToken":5,"kind":"file","atMs":5.25,"outcome":"exit-failure","exitCode":5,"osError":null}]});
        (value, backend)
    }
    #[test]
    fn permission_contract_preserves_failure_and_requires_exact_clock_stage_status() {
        let (good, backend) = witness();
        assert!(valid(&good, &backend, RUN, 1234));
        for (pointer, bad) in [
            ("/runId", json!("ffffffffffffffffffffffffffffffff")),
            ("/pid", json!(999)),
            ("/clockScope", json!("frontend")),
            ("/atMs", json!(11)),
            ("/valid", json!(false)),
            ("/maxCommands", json!(65)),
            ("/invalidReasons", json!(["private-text"])),
            ("/commands/1/kind", json!("private-path")),
            ("/commands/1/attemptId", json!(2)),
            ("/commands/1/stageToken", json!(3)),
            ("/commands/1/atMs", json!(6)),
            ("/commands/0/atMs", json!(2.5)),
            ("/commands/1/exitCode", json!(0)),
            ("/commands/1/exitCode", json!(2147483648_i64)),
            ("/commands/1/osError", json!(1.5)),
            ("/commands/1/outcome", json!("private-error")),
            ("/commands/0/outcome", json!("exit-failure")),
            ("/commands/1/exitCode", json!("private-error")),
        ] {
            let mut changed = good.clone();
            *changed.pointer_mut(pointer).unwrap() = bad;
            assert!(!valid(&changed, &backend, RUN, 1234), "{pointer}");
        }
        let mut leaked = good.clone();
        leaked["commands"][0]["path"] = json!("sentinel");
        assert!(!valid(&leaked, &backend, RUN, 1234));
        leaked = good.clone();
        leaked["username"] = json!("sentinel");
        assert!(!valid(&leaked, &backend, RUN, 1234));
        let mut incomplete = good.clone();
        incomplete["commands"].as_array_mut().unwrap().pop();
        assert!(!valid(&incomplete, &backend, RUN, 1234));
        let mut duplicate = good.clone();
        let copy = duplicate["commands"][0].clone();
        duplicate["commands"].as_array_mut().unwrap().push(copy);
        assert!(!valid(&duplicate, &backend, RUN, 1234));
        let mut overflow = good.clone();
        while overflow["commands"].as_array().unwrap().len() <= MAX_PERMISSION_COMMANDS {
            overflow["commands"]
                .as_array_mut()
                .unwrap()
                .push(copy_permission());
        }
        assert!(!valid(&overflow, &backend, RUN, 1234));
        let mut foreign = backend.clone();
        foreign["attempts"][0]["rootVerified"] = json!(false);
        assert!(!valid(&good, &foreign, RUN, 1234));
        assert!(!valid(&good, &backend, RUN, 0));
    }
    fn copy_permission() -> Value {
        witness().0["commands"][0].clone()
    }
    #[test]
    fn permission_contract_accepts_fixed_spawn_and_username_errors_without_text() {
        let (good, backend) = witness();
        for (name, code) in [
            ("spawn-error", json!(5)),
            ("spawn-error", Value::Null),
            ("username-unavailable", Value::Null),
            ("invalid-username", Value::Null),
        ] {
            let mut changed = good.clone();
            changed["commands"][1]["outcome"] = json!(name);
            changed["commands"][1]["exitCode"] = Value::Null;
            changed["commands"][1]["osError"] = code;
            assert!(valid(&changed, &backend, RUN, 1234));
        }
        let mut running = backend.clone();
        running["attempts"][0]["outcome"] = json!("running");
        running["attempts"][0]["endedAtMs"] = Value::Null;
        running["attempts"][0]["stages"][0]["outcome"] = json!("running");
        running["attempts"][0]["stages"][0]["endedAtMs"] = Value::Null;
        running["attempts"][0]["stages"][5]["outcome"] = json!("running");
        running["attempts"][0]["stages"][5]["endedAtMs"] = Value::Null;
        let mut waiting = good.clone();
        waiting["commands"].as_array_mut().unwrap().pop();
        assert!(valid(&waiting, &running, RUN, 1234));
    }
}
