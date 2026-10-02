use serde_json::json;
use solosoul_ipc_contract_gen::{generate, Generated};
use tempfile::TempDir;

// 直接编译共享纯 DTO 源，不链接插件运行时，也不复制 manifest 字段。
#[allow(dead_code)]
#[path = "../../../crates/solosoul-plugin/src/manifest.rs"]
mod manifest_fixture;
#[allow(dead_code)]
#[path = "../../../crates/solosoul-plugin/src/install_progress.rs"]
mod progress_fixture;

const HOST: &str = "src-tauri/src/commands/plugin.rs";
const PLUGIN_MANIFEST: &str = "crates/solosoul-plugin/Cargo.toml";
const PLUGIN_ROOT: &str = "crates/solosoul-plugin/src/mod.rs";
const SHARED: [(&str, &str); 4] = [
    (
        "crates/solosoul-plugin/src/manifest.rs",
        include_str!("../../../crates/solosoul-plugin/src/manifest.rs"),
    ),
    (
        "crates/solosoul-plugin/src/event.rs",
        include_str!("../../../crates/solosoul-plugin/src/event.rs"),
    ),
    (
        "crates/solosoul-plugin/src/install_progress.rs",
        include_str!("../../../crates/solosoul-plugin/src/install_progress.rs"),
    ),
    (
        "crates/solosoul-plugin/src/session.rs",
        include_str!("../../../crates/solosoul-plugin/src/session.rs"),
    ),
];
const PROGRESS: &str = r#"
#[derive(serde::Serialize)] #[serde(rename_all="camelCase")]
pub struct Progress { pub downloaded_bytes:u64, pub total_bytes:Option<u64> }
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
            "pub mod commands; fn dispatch(){tauri::generate_handler![commands::plugin::probe]}",
        );
        fixture.write("src-tauri/src/commands/mod.rs", "pub mod plugin;");
        fixture.write(HOST, source);
        fixture.write(
            "src-tauri/permissions/solo-soul/default.toml",
            "[[permission]]\nidentifier='allow-app-commands'\ncommands.allow=['probe']\n",
        );
        fixture
            .selection(json!({"commands":[{"name":"probe","source":HOST}],"types":[],"events":[]}));
        fixture
    }
    fn write(&self, relative: &str, text: &str) {
        let path = self.root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn selection(&self, value: serde_json::Value) {
        self.write("src-tauri/ipc-contracts.json", &value.to_string());
    }
    fn shared(&self) {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers=['src-tauri','crates/solosoul-plugin']\n",
        );
        self.write("src-tauri/Cargo.toml", "[package]\nname='host'\nversion='0.1.0'\n[dependencies]\nsolosoul-plugin={path='../crates/solosoul-plugin'}\n");
        self.write(
            PLUGIN_MANIFEST,
            include_str!("../../../crates/solosoul-plugin/Cargo.toml"),
        );
        self.write(
            PLUGIN_ROOT,
            include_str!("../../../crates/solosoul-plugin/src/mod.rs"),
        );
        for (path, source) in SHARED {
            self.write(path, source);
        }
        self.selection(json!({"commands":[{"name":"probe","source":HOST}],"types":SHARED.map(|(path,_)|path),"events":[],"crates":[PLUGIN_MANIFEST]}));
    }
    fn generated(&self) -> Generated {
        generate(self.root.path()).unwrap()
    }
    fn error(&self) -> String {
        generate(self.root.path()).unwrap_err()
    }
}
fn rejection(source: &str) {
    let fixture = Fixture::new(source);
    assert!(
        generate(fixture.root.path()).is_err(),
        "unexpected accepted contract: {source}"
    );
}
fn output(declaration: &str) -> String {
    Fixture::new(&format!(
        "{declaration}\n#[tauri::command] pub fn probe()->Payload{{unreachable!()}}"
    ))
    .generated()
    .typescript
}

#[test]
fn rf306_channel_uses_output_payload_and_resource_id_is_a_named_handle() {
    let source = format!("{PROGRESS}\nuse tauri::{{Webview,ResourceId,ipc::Channel}}; #[tauri::command] pub fn probe(webview:Webview,on_progress:Channel<Progress>,operation_id:Option<ResourceId>)->ResourceId{{unreachable!()}}");
    let fixture = Fixture::new(&source);
    let first = fixture.generated();
    assert_eq!(first.typescript, fixture.generated().typescript);
    assert_eq!(first.manifest, fixture.generated().manifest);
    assert_eq!(
        first
            .typescript
            .matches("import type { Channel as TauriChannel }")
            .count(),
        1
    );
    assert!(first.typescript.contains("from '@tauri-apps/api/core'"));
    assert!(first
        .typescript
        .contains("\"onProgress\": TauriChannel<Progress>;"));
    assert!(first
        .typescript
        .contains("\"operationId\"?: (ResourceId) | null;"));
    assert!(first.typescript.contains("result: ResourceId;"));
    assert!(first
        .typescript
        .contains("export type ResourceId = number;"));
    assert!(first
        .typescript
        .contains("\"totalBytes\": (number) | null;"));
    assert!(
        !first.typescript.contains("ProgressInput") && !first.typescript.contains("\"webview\"")
    );
    assert!(first
        .typescript
        .contains("export type IpcEvents = Record<never, never>;"));
    fixture.write(
        HOST,
        &source.replace("total_bytes:Option<u64>", "total_bytes:String"),
    );
    let changed = fixture.generated();
    assert_ne!(first.typescript, changed.typescript);
    assert!(changed.typescript.contains("\"totalBytes\": string;"));
    let plain =
        Fixture::new("#[tauri::command] pub fn probe()->String{unreachable!()}").generated();
    assert!(!plain.typescript.contains("TauriChannel") && !plain.typescript.contains("ResourceId"));
}

#[test]
fn rf306_real_shared_plugin_crate_and_all_fifteen_actual_command_signatures_generate() {
    let fixture = Fixture::new(include_str!("../../../src-tauri/src/commands/plugin.rs"));
    fixture.shared();
    // RF320：实际 Host 拒绝类型已迁移，fixture 同样显式注册真实 DTO。
    let error_source = "src-tauri/src/commands/error.rs";
    fixture.write(
        "src-tauri/src/commands/mod.rs",
        "pub mod plugin; pub mod error;",
    );
    fixture.write(
        error_source,
        include_str!("../../../src-tauri/src/commands/error.rs"),
    );
    let mut types: Vec<_> = SHARED.map(|(path, _)| path).into();
    types.push(error_source);
    let names = [
        "create_plugin_install",
        "plugin_list_all",
        "plugin_list_installed",
        "plugin_list_attachments",
        "plugin_install",
        "plugin_update",
        "plugin_uninstall",
        "plugin_run",
        "plugin_consent_response",
        "plugin_dialog_response",
        "plugin_list_sessions",
        "plugin_audit_log",
        "plugin_update_registry",
        "plugin_open_output_file",
        "plugin_copy_output_file",
    ];
    fixture.write(
        "src-tauri/src/lib.rs",
        &format!(
            "pub mod commands; fn dispatch(){{tauri::generate_handler![{}]}}",
            names
                .map(|name| format!("commands::plugin::{name}"))
                .join(",")
        ),
    );
    fixture.write(
        "src-tauri/permissions/solo-soul/default.toml",
        &format!(
            "[[permission]]\nidentifier='allow-app-commands'\ncommands.allow={}\n",
            serde_json::to_string(&names).unwrap()
        ),
    );
    fixture.selection(json!({"commands":names.map(|name|json!({"name":name,"source":HOST})),"types":types,"events":[],"crates":[PLUGIN_MANIFEST]}));
    let generated = fixture.generated();
    assert_eq!(generated.manifest["commands"].as_array().unwrap().len(), 15);
    assert_eq!(generated.manifest["unmigratedCommands"], json!([]));
    assert_eq!(generated.manifest["events"], json!([]));
    assert!(generated.manifest["sources"]
        .as_array()
        .unwrap()
        .contains(&json!(PLUGIN_ROOT)));
    assert_eq!(
        generated.manifest["structuredErrorCommands"]
            .as_array()
            .unwrap()
            .len(),
        14
    );
    let ts = &generated.typescript;
    assert!(ts.contains("\"plugin_run\": BackendError;"));
    assert!(ts.contains("\"create_plugin_install\": never;"));
    assert!(ts.contains("\"PLUGIN_CONSENT_DENIED\""));
    assert!(ts.contains("\"onProgress\": TauriChannel<PluginInstallProgress>;"));
    assert!(ts.contains("\"channel\": TauriChannel<PluginEvent>;"));
    assert!(ts.contains("export type PluginResultPayload = JsonValue;"));
    assert!(ts.contains("\"results\": Array<PluginResultPayload>;"));
    assert!(ts.contains("\"sessionId\": string;"));
    assert!(ts.contains("\"plugin_run_completed\" } & { \"exit_code\": number;"));
    assert!(ts.contains("\"displayName\"?: string;"));
    assert!(ts.contains("\"i18n\"?: Record<string, Record<string, string>>;"));
    assert!(!ts.contains("PluginInstallProgressInput") && !ts.contains("PluginEventInput"));
    assert!(!ts.contains(": any") && !ts.contains(": unknown"));
    let progress = progress_fixture::PluginInstallProgress {
        percent: 25,
        phase: progress_fixture::PluginInstallPhase::Downloading,
        downloaded_bytes: 16,
        total_bytes: None,
    };
    assert_eq!(
        serde_json::to_value(progress).unwrap(),
        json!({"percent":25,"phase":"downloading","downloadedBytes":16,"totalBytes":null})
    );
    assert!(ts.contains("\"totalBytes\": (number) | null;"));
}

#[test]
fn rf306_real_manifest_serialization_preserves_transparent_json_defaults_aliases_and_localized_strings(
) {
    let fixture=Fixture::new("use solosoul_plugin::manifest::PluginManifest; #[tauri::command] pub fn probe()->PluginManifest{unreachable!()}");
    fixture.shared();
    let ts = fixture.generated().typescript;
    assert!(ts.contains("\"dataTtlSeconds\": number;"));
    assert!(ts.contains("\"blockAllOutbound\": boolean;"));
    assert!(ts.contains("\"author\": (string) | null;"));
    assert!(ts.contains(
        "export type PluginParamType = \"string\" | \"number\" | \"boolean\" | \"select\";"
    ));
    assert!(ts.contains("export type PluginTier = \"p0\" | \"p1\" | \"p2\" | \"p3\" | \"p4\";"));
    let manifest:manifest_fixture::PluginManifest=serde_json::from_value(json!({"id":"synthetic","name":"合成","version":"1","description":"fixture","contracts":[{"typeId":"test/v1","displayName":{"zh":"合成契约","en":"Synthetic"},"roles":[{"roleId":"title","label":{"en":"Title"}}]}]})).unwrap();
    let wire = serde_json::to_value(&manifest).unwrap();
    assert_eq!(wire["dataTtlSeconds"], 300);
    assert_eq!(wire["tier"], "p3");
    assert!(wire.get("author").is_some() && wire["author"].is_null());
    assert!(wire.get("i18n").is_none() && wire.get("customUi").is_none());
    assert_eq!(wire["contracts"][0]["displayName"], "合成契约");
    assert_eq!(wire["contracts"][0]["roles"][0]["label"], "Title");
    assert_eq!(
        serde_json::to_value(manifest_fixture::PluginParamType::default()).unwrap(),
        "string"
    );
    let registry:manifest_fixture::RegistryEntry=serde_json::from_value(json!({"name":"synthetic","publisher":"author","versions":{"1":{"sha256":"synthetic","min_app_version":"1","max_app_version":"9"}}})).unwrap();
    let registry_wire = serde_json::to_value(registry).unwrap();
    assert_eq!(registry_wire["author"], "author");
    assert!(registry_wire.get("publisher").is_none());
    assert_eq!(registry_wire["versions"]["1"]["minAppVersion"], "1");
    for payload in [
        json!(null),
        json!(true),
        json!(3),
        json!("合成"),
        json!([null,{"nested":[1,false]}]),
        json!({"x":{"y":null}}),
    ] {
        let transparent = manifest_fixture::PluginResultPayload(payload.clone());
        assert_eq!(serde_json::to_value(&transparent).unwrap(), payload);
        assert_eq!(
            serde_json::from_value::<manifest_fixture::PluginResultPayload>(payload.clone())
                .unwrap()
                .0,
            payload
        );
    }
    let result = manifest_fixture::PluginResult {
        exit_code: 0,
        logs: vec![],
        results: vec![manifest_fixture::PluginResultPayload(json!({"safe":true}))],
        fuel_consumed: 7,
    };
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({"exitCode":0,"logs":[],"results":[{"safe":true}],"fuelConsumed":7})
    );
}

#[test]
fn rf306_transparent_json_supports_both_directions_without_array_wrapping() {
    let fixture=Fixture::new("#[derive(serde::Serialize,serde::Deserialize)] #[serde(transparent)] pub struct Payload(pub serde_json::Value); #[tauri::command] pub fn probe(payload:Payload)->Payload{unreachable!()}");
    let ts = fixture.generated().typescript;
    assert!(ts.contains("export type Payload = JsonValue;"));
    assert!(ts.contains("export type PayloadInput = JsonValue;"));
    assert!(!ts.contains("[JsonValue]"));
}

#[test]
fn rf306_deserialization_only_output_attributes_do_not_execute_or_change_wire_type() {
    let declaration = r#"#[derive(serde::Serialize,serde::Deserialize)] #[serde(default="panic_default")] pub struct Payload { #[serde(alias="legacy",alias="older",default="panic_default",deserialize_with="panic_decoder")] pub value:String } fn panic_default()->!{panic!("must not run")} fn panic_decoder()->!{panic!("must not run")}"#;
    assert!(output(declaration).contains("export type Payload = { \"value\": string; };"));
    rejection(&format!(
        "{declaration} #[tauri::command] pub fn probe(payload:Payload){{unreachable!()}}"
    ));
    for attribute in [r#"alias="legacy""#, r#"deserialize_with="decode_value""#] {
        rejection(&format!("#[derive(serde::Serialize,serde::Deserialize)] pub struct Payload{{#[serde({attribute})]pub value:String}} #[tauri::command] pub fn probe(payload:Payload){{unreachable!()}}"));
    }
    let fixture=Fixture::new("use solosoul_plugin::manifest::PluginManifest; #[tauri::command] pub fn probe(value:PluginManifest){unreachable!()}");
    fixture.shared();
    assert!(fixture.error().contains("only for output DTOs"));
}

#[test]
fn rf306_channel_lookalikes_missing_and_wrong_type_arguments_are_rejected() {
    for ty in [
        "Channel<Progress>",
        "other::Channel<Progress>",
        "tauri::Channel<Progress>",
        "tauri::ipc::Channel",
        "tauri::ipc::Channel<Progress,String>",
        "tauri::ipc::Channel<'static>",
        "tauri::ipc::Channel<{1}>",
        "tauri::ipc::Channel<tauri::ipc::Channel<Progress>>",
    ] {
        rejection(&format!(
            "{PROGRESS} #[tauri::command] pub fn probe(channel:{ty}){{unreachable!()}}"
        ));
    }
}

#[test]
fn rf306_channel_is_rejected_in_nested_arguments_dtos_results_and_event_payloads() {
    for ty in [
        "Option<tauri::ipc::Channel<Progress>>",
        "Vec<tauri::ipc::Channel<Progress>>",
    ] {
        rejection(&format!(
            "{PROGRESS} #[tauri::command] pub fn probe(channel:{ty}){{unreachable!()}}"
        ));
    }
    rejection(&format!("{PROGRESS} #[derive(serde::Deserialize)] pub struct Payload{{channel:tauri::ipc::Channel<Progress>}} #[tauri::command] pub fn probe(value:Payload){{unreachable!()}}"));
    rejection(&format!("{PROGRESS} #[derive(serde::Serialize)] pub struct Payload{{channel:tauri::ipc::Channel<Progress>}} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"));
    rejection(&format!("{PROGRESS} #[tauri::command] pub fn probe()->tauri::ipc::Channel<Progress>{{unreachable!()}}"));
    let fixture = Fixture::new(&format!(
        "{PROGRESS} #[tauri::command] pub fn probe(){{unreachable!()}}"
    ));
    fixture.selection(json!({"commands":[{"name":"probe","source":HOST}],"types":[],"events":[{"name":"bad-channel","source":HOST,"payload":"tauri::ipc::Channel<Progress>"}]}));
    assert!(fixture.error().contains("unsupported generic type"));
}

#[test]
fn rf306_channel_payload_requires_serialize_even_if_deserialize_exists() {
    rejection("#[derive(serde::Deserialize)] pub struct Progress{value:String} #[tauri::command] pub fn probe(channel:tauri::ipc::Channel<Progress>){unreachable!()}");
    let fixture=Fixture::new(&format!("{PROGRESS} #[tauri::command] pub fn probe(channel:tauri::ipc::Channel<Progress>){{unreachable!()}}"));
    assert!(fixture
        .generated()
        .typescript
        .contains("TauriChannel<Progress>"));
}

#[test]
fn rf306_webview_and_resource_names_cannot_disguise_unknown_wire_types() {
    for ty in [
        "Webview",
        "other::Webview",
        "tauri::Webview<Runtime>",
        "tauri::Resource",
        "ResourceId",
        "other::ResourceId",
        "tauri::ResourceId<String>",
    ] {
        rejection(&format!(
            "#[tauri::command] pub fn probe(value:{ty}){{unreachable!()}}"
        ));
    }
    rejection("#[command] pub fn probe(view:tauri::Webview){unreachable!()}");
    rejection("use tauri::command as macro_alias; #[macro_alias] pub fn probe(){unreachable!()}");
    rejection("mod tauri {pub struct ResourceId;} #[tauri::command] pub fn probe()->tauri::ResourceId{unreachable!()}");
    let fixture=Fixture::new("use tauri::ResourceId as Handle; use tauri::Webview as View; #[tauri::command] pub fn probe(view:View,handle:Handle)->Handle{unreachable!()}");
    assert!(fixture
        .generated()
        .typescript
        .contains("args: { \"handle\": ResourceId; }; result: ResourceId;"));
}

#[test]
fn rf306_generated_helper_names_cannot_be_shadowed_by_dtos() {
    for name in ["ResourceId", "TauriChannel"] {
        rejection(&format!("#[derive(serde::Serialize)] pub struct {name}{{value:String}} #[tauri::command] pub fn probe()->{name}{{unreachable!()}}"));
        rejection(&format!("#[derive(serde::Serialize)] pub struct {name}{{value:String}} #[tauri::command] pub fn probe(channel:tauri::ipc::Channel<{name}>){{unreachable!()}}"));
    }
}

#[test]
fn rf306_transparent_rejects_multiple_named_unit_and_custom_field_shapes() {
    for declaration in [
        "#[serde(transparent)] pub struct Payload;",
        "#[serde(transparent)] pub struct Payload();",
        "#[serde(transparent)] pub struct Payload(String,String);",
        "#[serde(transparent)] pub struct Payload{value:String}",
        "pub struct Payload(serde_json::Value);",
        "#[serde(transparent)] pub struct Payload(#[serde(skip)] String);",
        "#[serde(transparent)] pub struct Payload(#[serde(serialize_with=\"custom\")] String);",
        "#[serde(transparent,default)] pub struct Payload(String);",
    ] {
        rejection(&format!("#[derive(serde::Serialize)]{declaration} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"));
    }
}

#[test]
fn rf306_custom_serialization_and_unknown_serde_remain_fail_closed() {
    for attribute in [
        "flatten",
        "untagged",
        "skip",
        "with=\"codec\"",
        "serialize_with=\"codec\"",
        "skip_serializing_if=\"custom::empty\"",
        "rename(serialize=\"a\",deserialize=\"b\")",
    ] {
        rejection(&format!("#[derive(serde::Serialize)] pub struct Payload{{#[serde({attribute})] value:String}} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"));
    }
    rejection("#[derive(serde::Serialize)] #[serde(transparent)] pub struct Payload(serde_json::Value); impl serde::Serialize for Payload{} #[tauri::command] pub fn probe()->Payload{unreachable!()}");
    rejection("#[derive(serde::Serialize)] pub struct Payload{value:String} impl serde::Deserialize for Payload{} #[tauri::command] pub fn probe()->Payload{unreachable!()}");
}

#[test]
fn rf306_malformed_deserialization_attributes_and_invalid_locations_are_rejected() {
    for attribute in [
        "alias=3",
        "default=\"call()\"",
        "default=\"\"",
        "default=\"generic::<String>\"",
        "deserialize_with=\"not a path\"",
        "deserialize_with=3",
        "default(function)",
    ] {
        rejection(&format!("#[derive(serde::Serialize)] pub struct Payload{{#[serde({attribute})]value:String}} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"));
    }
    for attribute in ["alias=\"legacy\"", "deserialize_with=\"decoder\""] {
        rejection(&format!("#[derive(serde::Serialize)] #[serde({attribute})] pub struct Payload{{value:String}} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"));
    }
}

#[test]
fn rf306_default_variant_is_only_one_unit_marker_on_a_default_enum() {
    let declaration="#[derive(serde::Serialize,serde::Deserialize,Default)] pub enum Payload{#[default] First,Second}";
    assert!(output(declaration).contains("export type Payload = \"First\" | \"Second\";"));
    for declaration in [
        "#[derive(serde::Serialize)] pub enum Payload{#[default] First}",
        "#[derive(serde::Serialize,Default)] pub enum Payload{#[default] First,#[default] Second}",
        "#[derive(serde::Serialize,Default)] pub enum Payload{#[default] First(String)}",
        "#[derive(serde::Serialize,Default)] pub enum Payload{#[default] First{value:String}}",
        "#[derive(serde::Serialize,Default)] pub enum Payload{#[default()] First}",
        "#[derive(serde::Serialize,Default)] pub enum Payload{#[default] #[default] First}",
        "#[derive(serde::Serialize,Default)] #[default] pub struct Payload{value:String}",
        "#[derive(serde::Serialize,Default)] pub struct Payload{#[default] value:String}",
    ] {
        rejection(&format!(
            "{declaration} #[tauri::command] pub fn probe()->Payload{{unreachable!()}}"
        ));
    }
}

#[test]
fn rf306_plugin_crate_identity_cannot_be_replaced_or_implicitly_registered() {
    let fixture=Fixture::new("use solosoul_plugin::manifest::PluginManifest; #[tauri::command] pub fn probe()->PluginManifest{unreachable!()}");
    fixture.shared();
    fixture.selection(json!({"commands":[{"name":"probe","source":HOST}],"types":SHARED.map(|(path,_)|path),"events":[]}));
    assert!(fixture.error().contains("outside a registered crate"));
    fixture.shared();
    let manifest = include_str!("../../../crates/solosoul-plugin/Cargo.toml");
    fixture.write(
        PLUGIN_MANIFEST,
        &manifest.replace("name = \"solosoul_plugin\"", "name = \"lookalike\""),
    );
    assert!(generate(fixture.root.path()).is_err());
    fixture.shared();
    fixture.write(
        PLUGIN_ROOT,
        "pub mod event; pub mod install_progress; pub mod session;",
    );
    assert!(fixture.error().contains("exactly one declaration"));
    fixture.shared();
    fixture.write(HOST,"mod solosoul_plugin { pub mod manifest; } #[tauri::command] pub fn probe()->solosoul_plugin::manifest::PluginManifest{unreachable!()}");
    assert!(fixture.error().contains("shadows known contract namespace"));
}

#[test]
fn rf306_use_alias_group_and_name_cannot_shadow_sdk_namespace() {
    let fake = "pub mod fake { pub use ::tauri::command; pub type ResourceId=String; pub type Webview=String; pub mod ipc {pub type Channel<T>=Option<T>;} pub mod tauri {pub use ::tauri::command; pub type ResourceId=String; pub type Webview=String; pub mod ipc {pub type Channel<T>=Option<T>;}} }";
    for binding in [
        "use crate::fake as tauri;",
        "use crate::{fake as tauri};",
        "use crate::{fake::{tauri}};",
        "use crate::fake::tauri;",
        "use crate::fake as r#tauri;",
    ] {
        for signature in [
            "use tauri::ResourceId; #[tauri::command] pub fn probe()->ResourceId{unreachable!()}",
            "use tauri::Webview; #[tauri::command] pub fn probe(input:Webview){unreachable!()}",
            "use tauri::ipc::Channel; #[tauri::command] pub fn probe(channel:Channel<String>){unreachable!()}",
        ] {
            let fixture=Fixture::new(&format!("{binding} {signature}"));
            fixture.write("src-tauri/src/lib.rs",&format!("{fake} pub mod commands; fn dispatch(){{tauri::generate_handler![commands::plugin::probe]}}"));
            assert!(fixture.error().contains("shadows known contract namespace"), "binding must be rejected: {binding} {signature}");
        }
    }
}

#[test]
fn rf306_namespace_checks_include_ancestor_self_and_glob_imports() {
    for relative in ["src-tauri/src/lib.rs", "src-tauri/src/commands/mod.rs"] {
        let base = if relative.ends_with("lib.rs") {
            "pub mod commands; fn dispatch(){tauri::generate_handler![commands::plugin::probe]}"
        } else {
            "pub mod plugin;"
        };
        for binding in [
            "use crate::fake as tauri;",
            "use crate::fake::{tauri};",
            "use crate::fake::tauri::{self};",
            "use crate::fake::{tauri::{self}};",
            "use crate::fake::tauri::{self as tauri};",
            "use crate::fake::*;",
            "use crate::fake::{nested::*};",
        ] {
            let fixture = Fixture::new("#[tauri::command] pub fn probe()->String{unreachable!()}");
            fixture.write(relative, &format!("{binding} {base}"));
            let error = fixture.error();
            assert!(
                error.contains("shadows known contract namespace")
                    || error.contains("use self imports are unsupported")
                    || error.contains("glob imports are unsupported"),
                "wrong rejection for {relative} {binding}: {error}"
            );
        }
    }
}

#[test]
fn rf306_use_rebindings_cannot_spoof_standard_or_registered_external_types() {
    for namespace in ["std", "core", "alloc", "serde", "serde_json"] {
        for binding in [
            format!("use crate::fake as {namespace};"),
            format!("use crate::fake::{{{namespace}}};"),
        ] {
            let fixture = Fixture::new(&format!(
                "{binding} #[tauri::command] pub fn probe()->String{{unreachable!()}}"
            ));
            assert!(fixture.error().contains("shadows known contract namespace"));
        }
    }
    for binding in [
        "use crate::fake as solosoul_plugin;",
        "use crate::{fake as solosoul_plugin};",
        "use crate::fake::{solosoul_plugin};",
    ] {
        let fixture=Fixture::new(&format!("{binding} use solosoul_plugin::manifest::PluginManifest; #[tauri::command] pub fn probe()->PluginManifest{{unreachable!()}}"));
        fixture.shared();
        assert!(fixture.error().contains("shadows known contract namespace"));
        // 同样验证不加入 Catalog 的 Host 祖先文件。
        fixture.write(
            HOST,
            "#[tauri::command] pub fn probe()->String{unreachable!()}",
        );
        fixture.write(
            "src-tauri/src/commands/mod.rs",
            &format!("{binding} pub mod plugin;"),
        );
        assert!(fixture.error().contains("shadows known contract namespace"));
    }
}

#[test]
fn rf306_genuine_sdk_leaf_aliases_and_nested_groups_remain_supported() {
    let source = format!("{PROGRESS} use tauri::{{ipc::{{Channel as ProgressChannel}},ResourceId as Handle,Webview as View}}; use std::collections::HashMap as Parameters; #[tauri::command] pub fn probe(view:View,channel:ProgressChannel<Progress>,handle:Handle,params:Parameters<String,String>)->Handle{{unreachable!()}}");
    let fixture = Fixture::new(&source);
    let ts = fixture.generated().typescript;
    assert!(ts.contains("\"channel\": TauriChannel<Progress>;"));
    assert!(ts.contains("\"handle\": ResourceId;"));
    assert!(ts.contains("\"params\": Record<string, string>;"));
    assert!(ts.contains("result: ResourceId;"));
    assert!(!ts.contains("\"view\""));
}
