use serde_json::json;
use solosoul_ipc_contract_gen::{generate, Generated};
use tempfile::TempDir;

#[path = "fixtures/dtos.rs"]
mod dto_fixture;

const SYSTEM: &str = "src-tauri/src/commands/system.rs";
const BASE: &str = r#"
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo { pub app_name: String, pub version: String, pub os: String, pub arch: String }
#[tauri::command]
pub async fn get_app_info() -> Result<AppInfo, String> { panic!("must never execute business") }
#[tauri::command]
pub fn legacy_ping() -> String { unreachable!() }
"#;

struct Fixture {
    root: TempDir,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fixture.write(
            "src-tauri/src/lib.rs",
            r#"
pub mod commands;
fn core() { tauri::generate_handler![commands::system::get_app_info] }
fn second_cluster() { tauri::generate_handler![commands::system::legacy_ping,] }
#[cfg(test)] mod tests { fn example() { tauri::generate_handler![commands::system::not_real] } }
const DOCUMENTATION: &str = "tauri::generate_handler![commands::system::not_real]";
"#,
        );
        fixture.write("src-tauri/src/commands/mod.rs", "pub mod system;");
        fixture.write(SYSTEM, source);
        fixture.acl(&["get_app_info", "legacy_ping"]);
        fixture.selection(
            json!({"commands":[{"name":"get_app_info","source":SYSTEM}],"types":[],"events":[]}),
        );
        fixture
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn acl(&self, commands: &[&str]) {
        self.write(
            "src-tauri/permissions/solo-soul/default.toml",
            &format!(
                "[[permission]]\nidentifier = \"allow-app-commands\"\ncommands.allow = {}\n",
                serde_json::to_string(commands).unwrap()
            ),
        );
    }
    fn selection(&self, value: serde_json::Value) {
        self.write("src-tauri/ipc-contracts.json", &value.to_string());
    }
    fn generate(&self) -> Generated {
        generate(self.root.path()).unwrap()
    }
    fn error(&self) -> String {
        generate(self.root.path()).unwrap_err()
    }
}

#[test]
fn rf301_real_signature_and_all_handler_clusters_produce_deterministic_inventory() {
    let fixture = Fixture::new(BASE);
    let before = std::fs::read(fixture.root.path().join(SYSTEM)).unwrap();
    let first = fixture.generate();
    let second = fixture.generate();
    assert_eq!(first.typescript, second.typescript);
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.manifest["commands"], json!(["get_app_info"]));
    assert_eq!(first.manifest["unmigratedCommands"], json!(["legacy_ping"]));
    assert_eq!(first.manifest["events"], json!([]));
    assert_eq!(first.manifest["schemaVersion"], 1);
    assert_eq!(first.manifest["generator"]["version"], "0.1.0");
    assert!(first
        .typescript
        .contains("\"get_app_info\": { args: undefined; result: AppInfo; }"));
    assert!(first.typescript.contains("\"appName\": string;"));
    assert!(first
        .typescript
        .contains("export type IpcEvents = Record<never, never>;"));
    assert!(!first.typescript.contains("not_real"));
    assert_eq!(
        before,
        std::fs::read(fixture.root.path().join(SYSTEM)).unwrap(),
        "generation is read-only"
    );
    assert_eq!(
        first.manifest["sources"],
        json!([
            "src-tauri/ipc-contracts.json",
            "src-tauri/permissions/solo-soul/default.toml",
            "src-tauri/src/commands/mod.rs",
            "src-tauri/src/commands/system.rs",
            "src-tauri/src/lib.rs"
        ])
    );
}

#[test]
fn rf301_mutating_actual_parameters_and_return_changes_contract_without_editing_selection() {
    let fixture = Fixture::new(BASE);
    let old = fixture.generate();
    fixture.write(
        SYSTEM,
        &BASE.replace(
            "get_app_info()",
            "get_app_info(account_id: String, page_size: Option<u32>)",
        ),
    );
    let params = fixture.generate();
    assert_ne!(params.typescript, old.typescript);
    assert!(params.typescript.contains("\"accountId\": string;"));
    assert!(params
        .typescript
        .contains("\"pageSize\"?: (number) | null;"));
    fixture.write(
        SYSTEM,
        &BASE.replace("Result<AppInfo, String>", "Result<Vec<AppInfo>, String>"),
    );
    let response = fixture.generate();
    assert_ne!(response.typescript, old.typescript);
    assert!(response.typescript.contains("result: Array<AppInfo>"));
    fixture.write(
        SYSTEM,
        &BASE.replace("Result<AppInfo, String>", "Result<UnknownResponse, String>"),
    );
    assert!(fixture
        .error()
        .contains("unknown or unregistered wire type"));
}

#[test]
fn rf301_serde_direction_nested_dtos_and_enum_names_match_same_source_serialization() {
    let source = format!("{}\n#[tauri::command]\npub fn get_app_info(payload: Payload) -> Result<Payload, String> {{ unreachable!() }}", include_str!("fixtures/dtos.rs"));
    let fixture = Fixture::new(&source);
    let output = fixture.generate();
    let ts = output.typescript;
    let out = ts
        .lines()
        .find(|line| line.starts_with("export type Payload ="))
        .unwrap();
    let input = ts
        .lines()
        .find(|line| line.starts_with("export type PayloadInput ="))
        .unwrap();
    assert!(out.contains("\"explicit-id\": string;"));
    assert!(out.contains("\"parentId\": (string) | null;"));
    assert!(out.contains("\"contractId\"?: string;"));
    assert!(out.contains("\"sequence\": number;"));
    assert!(out.contains("\"nested\": Array<Nested>;"));
    assert!(input.contains("\"parentId\"?: (string) | null;"));
    assert!(input.contains("\"contractId\"?: (string) | null;"));
    assert!(input.contains("\"sequence\"?: number;"));
    assert!(input.contains("Array<NestedInput>"));
    assert!(
        ts.contains("\"ready\" | \"h_t_t_p_ready\" | \"completed\""),
        "serde variants must not use heck acronym grouping"
    );
    let value = dto_fixture::Payload {
        id: "synthetic".into(),
        parent_id: None,
        contract_id: None,
        sequence: 7,
        nested: vec![dto_fixture::Nested {
            display_name: "合成 🌍".into(),
        }],
        state: dto_fixture::State::HTTPReady,
    };
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serialized,
        json!({"explicit-id":"synthetic","parentId":null,"sequence":7,"nested":[{"displayName":"合成 🌍"}],"state":"h_t_t_p_ready"})
    );
    let deserialized: dto_fixture::Payload =
        serde_json::from_value(json!({"explicit-id":"synthetic","nested":[],"state":"ready"}))
            .unwrap();
    assert_eq!(deserialized.sequence, 0);
    assert!(deserialized.parent_id.is_none() && deserialized.contract_id.is_none());
    let populated = serde_json::to_value(dto_fixture::Payload {
        contract_id: Some("type-a".into()),
        ..value
    })
    .unwrap();
    assert_eq!(populated["contractId"], "type-a");
}

#[test]
fn rf301_explicit_events_and_three_enum_representations_are_generated_from_rust() {
    let source = format!("{}\n#[tauri::command] pub fn get_app_info() -> Result<String, String> {{ unreachable!() }}", include_str!("fixtures/dtos.rs"));
    let fixture = Fixture::new(&source);
    fixture.selection(
        json!({"commands":[{"name":"get_app_info","source":SYSTEM}],"types":[],"events":[
            {"name":"z-external","source":SYSTEM,"payload":"External"},
            {"name":"a-progress","source":SYSTEM,"payload":"Event"},
            {"name":"internal","source":SYSTEM,"payload":"Internal"}
        ]}),
    );
    let output = fixture.generate();
    assert_eq!(
        output.manifest["events"],
        json!(["a-progress", "internal", "z-external"])
    );
    assert!(output.typescript.contains("\"a-progress\": Event;"));
    assert!(output
        .typescript
        .contains("{ \"kind\": \"updated\"; \"payload\": Payload }"));
    assert!(output
        .typescript
        .contains("({ \"kind\": \"failed\" } & { \"reason\": string; })"));
    assert!(output
        .typescript
        .contains("{ \"pair\": [string, boolean] }"));
    assert_eq!(
        serde_json::to_value(dto_fixture::Event::Idle).unwrap(),
        json!({"kind":"idle"})
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::Event::Pair("x".into(), 2)).unwrap(),
        json!({"kind":"pair","payload":["x",2]})
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::Event::Failed {
            error_message: "bad".into()
        })
        .unwrap(),
        json!({"kind":"failed","payload":{"errorMessage":"bad"}})
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::Internal::Failed {
            reason: "bad".into()
        })
        .unwrap(),
        json!({"kind":"failed","reason":"bad"})
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::External::Pair("x".into(), true)).unwrap(),
        json!({"pair":["x",true]})
    );
}

#[test]
fn rf301_handler_path_and_module_declarations_prevent_same_name_source_substitution() {
    let fixture = Fixture::new(BASE);
    fixture.write("src-tauri/src/commands/other.rs", BASE);
    fixture.selection(json!({"commands":[{"name":"get_app_info","source":"src-tauri/src/commands/other.rs"}],"types":[],"events":[]}));
    assert!(fixture.error().contains("does not match handler module"));
    fixture.selection(
        json!({"commands":[{"name":"get_app_info","source":SYSTEM}],"types":[],"events":[]}),
    );
    fixture.write("src-tauri/src/commands/mod.rs", "pub mod other;");
    assert!(fixture.error().contains("exactly one declaration"));
}

#[test]
fn rf301_duplicate_registration_acl_selection_and_unknown_commands_fail_closed() {
    let fixture = Fixture::new(BASE);
    fixture.acl(&["get_app_info"]);
    assert!(fixture
        .error()
        .contains("registered command missing ACL: legacy_ping"));
    fixture.acl(&["get_app_info", "legacy_ping", "get_app_info"]);
    assert!(fixture.error().contains("duplicate ACL command"));
    fixture.acl(&["get_app_info", "legacy_ping"]);
    fixture
        .selection(json!({"commands":[{"name":"unknown","source":SYSTEM}],"types":[],"events":[]}));
    assert!(fixture.error().contains("not registered"));
    fixture.selection(json!({"commands":[{"name":"get_app_info","source":SYSTEM},{"name":"get_app_info","source":SYSTEM}],"types":[],"events":[]}));
    assert!(fixture.error().contains("duplicate selected command"));
    fixture.write("src-tauri/src/lib.rs", "pub mod commands; fn a(){tauri::generate_handler![commands::system::get_app_info,commands::system::get_app_info]}");
    assert!(fixture.error().contains("duplicate handler command"));
}

#[test]
fn rf301_unsupported_rust_and_serde_never_degrade_to_any() {
    let declarations = [
        "#[derive(serde::Serialize)] pub struct AppInfo<T> { pub value: T }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(flatten)] pub value: String }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(with=\"custom\")] pub value: String }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(rename(serialize=\"a\",deserialize=\"b\"))] pub value: String }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[cfg(windows)] pub value: String }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(default=\"load_secret\")] pub value: String }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(skip_serializing_if=\"Vec::is_empty\")] pub value: Vec<String> }",
        "#[derive(serde::Serialize)] pub struct AppInfo { #[serde(skip_serializing_if=\"Option::is_none\")] pub value: String }",
        "#[derive(serde::Serialize)] #[serde(untagged)] pub enum AppInfo { Text(String) }",
        "pub type AppInfo = String;",
        "#[derive(serde::Serialize)] pub struct AppInfo { pub value: serde_json::Value }",
        "#[derive(serde::Serialize)] pub struct AppInfo { pub value: [u8; 3] }",
        "#[derive(serde::Serialize)] pub struct AppInfo { pub value: Unknown }",
        "pub struct AppInfo { pub value: String } impl serde::Serialize for AppInfo { }",
        "#[derive(serde::Serialize)] #[serde(tag=\"kind\")] pub enum AppInfo { Text(String) }",
    ];
    for declaration in declarations {
        let fixture = Fixture::new(&format!("{declaration}\n#[tauri::command] pub fn get_app_info() -> Result<AppInfo,String> {{ unreachable!() }}"));
        assert!(
            generate(fixture.root.path()).is_err(),
            "unsupported declaration accepted: {declaration}"
        );
    }
}

#[test]
fn rf301_input_requires_deserialize_and_unknown_command_attributes_are_rejected() {
    let fixture = Fixture::new(&BASE.replace("get_app_info()", "get_app_info(input: AppInfo)"));
    assert!(fixture.error().contains("Deserialize derive is required"));
    for source in [
        BASE.replace(
            "pub async fn get_app_info()",
            "pub async fn get_app_info<T>(value:T)",
        ),
        BASE.replace(
            "#[tauri::command]\npub async",
            "#[tauri::command(rename = \"other\")]\npub async",
        ),
        BASE.replace(
            "pub async fn get_app_info()",
            "pub async fn get_app_info(value: &str)",
        ),
        BASE.replace("Result<AppInfo, String>", "Result<AppInfo, anyhow::Error>"),
        BASE.replace(
            "#[tauri::command]\npub async",
            "#[cfg(windows)]\n#[tauri::command]\npub async",
        ),
    ] {
        fixture.write(SYSTEM, &source);
        assert!(
            generate(fixture.root.path()).is_err(),
            "unsupported signature/attribute accepted"
        );
    }
}

#[test]
fn rf301_command_case_option_parameters_and_known_tauri_injection_follow_real_signature() {
    let source = BASE.replace("get_app_info()", "get_app_info(state: tauri::State<'_, AppState>, app: tauri::AppHandle, http_status: String, maybe_count: Option<u32>)");
    let fixture = Fixture::new(&source);
    let ts = fixture.generate().typescript;
    assert!(ts.contains("\"httpStatus\": string;"));
    assert!(ts.contains("\"maybeCount\"?: (number) | null;"));
    assert!(!ts.contains("\"state\":") && !ts.contains("\"app\":"));
    fixture.write(
        SYSTEM,
        &source.replace(
            "#[tauri::command]\npub async",
            "#[tauri::command(rename_all=\"snake_case\")]\npub async",
        ),
    );
    assert!(fixture
        .generate()
        .typescript
        .contains("\"http_status\": string;"));
    fixture.write(
        SYSTEM,
        &source.replace("tauri::State<'_, AppState>", "LookalikeState<AppState>"),
    );
    assert!(fixture.error().contains("unsupported generic type"));
}

#[test]
fn rf301_extra_dto_source_is_resolved_from_actual_module_and_import() {
    let fixture = Fixture::new(
        r#"use crate::dto::AppInfo as Response; #[tauri::command] pub fn get_app_info() -> Result<Response,String> { unreachable!() }"#,
    );
    let lib = std::fs::read_to_string(fixture.root.path().join("src-tauri/src/lib.rs")).unwrap();
    fixture.write("src-tauri/src/lib.rs", &format!("pub mod dto;\n{lib}"));
    fixture.write(
        "src-tauri/src/dto.rs",
        "#[derive(serde::Serialize)] pub struct AppInfo { pub value:String }",
    );
    fixture.selection(json!({"commands":[{"name":"get_app_info","source":SYSTEM}],"types":["src-tauri/src/dto.rs"],"events":[]}));
    let output = fixture.generate();
    assert!(output
        .typescript
        .contains("export type AppInfo = { \"value\": string; };"));
    assert!(output.manifest["sources"]
        .as_array()
        .unwrap()
        .contains(&json!("src-tauri/src/dto.rs")));
    fixture.selection(
        json!({"commands":[{"name":"get_app_info","source":SYSTEM}],"types":[],"events":[]}),
    );
    assert!(fixture
        .error()
        .contains("unknown or unregistered wire type"));
}

#[test]
fn rf301_duplicate_events_and_reserved_serialized_names_fail() {
    let fixture = Fixture::new(BASE);
    fixture.selection(json!({"commands":[],"types":[],"events":[
        {"name":"same","source":SYSTEM,"payload":"AppInfo"}, {"name":"same","source":SYSTEM,"payload":"AppInfo"}
    ]}));
    assert!(fixture.error().contains("duplicate event"));
    let fixture = Fixture::new("#[derive(serde::Serialize)] pub struct AppInfo { #[serde(rename=\"x\")] pub a:String, #[serde(rename=\"x\")] pub b:String } #[tauri::command] pub fn get_app_info()->Result<AppInfo,String>{unreachable!()}");
    assert!(fixture
        .error()
        .contains("duplicate/reserved serialized field"));
    let fixture = Fixture::new("#[derive(serde::Serialize)] #[serde(tag=\"kind\")] pub enum AppInfo { A { kind:String } } #[tauri::command] pub fn get_app_info()->Result<AppInfo,String>{unreachable!()}");
    assert!(fixture
        .error()
        .contains("duplicate/reserved serialized field"));
}

#[test]
fn rf301_paths_and_selection_cannot_supply_handwritten_response_contracts() {
    let fixture = Fixture::new(BASE);
    for source in [
        "../outside.rs",
        "src-tauri/src/commands/../system.rs",
        "C:/outside.rs",
        "src-tauri/src/commands/system.txt",
    ] {
        fixture.selection(json!({"commands":[],"types":[source],"events":[]}));
        assert!(
            generate(fixture.root.path()).is_err(),
            "accepted unsafe/non-Rust path: {source}"
        );
    }
    fixture.selection(json!({"commands":[{"name":"get_app_info","source":SYSTEM,"result":"string"}],"types":[],"events":[]}));
    assert!(fixture.error().contains("unknown field"));
}

#[test]
fn rf301_executable_stdout_is_only_json_and_does_not_run_business() {
    let fixture = Fixture::new(BASE);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_solosoul-ipc-contract-gen"))
        .arg("--root")
        .arg(fixture.root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let output: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output["typescript"].as_str().unwrap().contains("AppInfo"));
    assert_eq!(output["manifest"]["commands"], json!(["get_app_info"]));
    assert!(!fixture.root.path().join("generated.ts").exists());
}

#[test]
fn rf301_float_serialization_and_empty_object_have_direction_correct_types() {
    let fixture = Fixture::new(&format!("{}\n#[tauri::command] pub fn get_app_info(empty:Empty, float:Floating) -> Result<Floating,String> {{unreachable!()}}",include_str!("fixtures/dtos.rs")));
    let ts = fixture.generate().typescript;
    assert!(ts.contains("export type EmptyInput = Record<string, never>;"));
    assert!(ts.contains(
        "export type Floating = { \"score\": number | null; \"weight\": number | null; };"
    ));
    assert!(ts.contains("export type FloatingInput = { \"score\": number; \"weight\": number; };"));
    let value = dto_fixture::Floating {
        score: f64::NAN,
        weight: f32::INFINITY,
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({"score":null,"weight":null})
    );
    assert!(
        serde_json::from_value::<dto_fixture::Floating>(json!({"score":null,"weight":1})).is_err()
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::Empty {}).unwrap(),
        json!({})
    );
    assert!(serde_json::from_value::<dto_fixture::Empty>(json!(1)).is_err());
}

#[test]
fn rf301_platform_condition_on_handler_container_cannot_silently_migrate() {
    let fixture = Fixture::new(BASE);
    fixture.write("src-tauri/src/lib.rs", "pub mod commands; #[cfg(windows)] fn core(){tauri::generate_handler![commands::system::get_app_info]} fn rest(){tauri::generate_handler![commands::system::legacy_ping]}");
    assert!(fixture.error().contains("conditional migrated command"));
}

#[test]
fn rf301_internally_tagged_empty_named_variant_matches_same_source_serde() {
    let fixture = Fixture::new(&format!("{}\n#[tauri::command] pub fn get_app_info(value:EmptyInternal) -> Result<EmptyInternal,String> {{unreachable!()}}", include_str!("fixtures/dtos.rs")));
    let ts = fixture.generate().typescript;
    let expected = "{ \"kind\": \"empty\" } | { \"kind\": \"unit\" }";
    assert!(ts.contains(&format!("export type EmptyInternal = {expected};")));
    assert!(ts.contains(&format!("export type EmptyInternalInput = {expected};")));
    let value = dto_fixture::EmptyInternal::Empty {};
    let wire = serde_json::to_value(&value).unwrap();
    assert_eq!(wire, json!({"kind":"empty"}));
    assert_eq!(
        serde_json::from_value::<dto_fixture::EmptyInternal>(wire).unwrap(),
        value
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::EmptyInternal::Unit).unwrap(),
        json!({"kind":"unit"})
    );
}

#[test]
fn rf301_dto_outputs_cannot_shadow_typescript_helpers() {
    assert_eq!(
        serde_json::to_value(dto_fixture::Array {
            value: "array".into()
        })
        .unwrap(),
        json!({"value":"array"})
    );
    assert_eq!(
        serde_json::to_value(dto_fixture::Record {
            value: "record".into()
        })
        .unwrap(),
        json!({"value":"record"})
    );
    for name in ["Array", "Record"] {
        let fixture = Fixture::new(&format!("{}\n#[tauri::command] pub fn get_app_info() -> Result<{name},String> {{unreachable!()}}", include_str!("fixtures/dtos.rs")));
        assert!(fixture
            .error()
            .contains(&format!("unsupported TypeScript export name: {name}")));
    }
}
