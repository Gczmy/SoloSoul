use serde_json::json;
use solosoul_ipc_contract_gen::{generate, Generated};
use tempfile::TempDir;

#[path = "fixtures/rf302_external.rs"]
mod external_fixture;
#[path = "fixtures/rf302_dtos.rs"]
mod host_fixture;
// 与外部夹具中的 crate::SharedLeaf 共源编译，不链接 Vault。
pub use external_fixture::SharedLeaf;

const HOST: &str = "src-tauri/src/commands/object.rs";
const VAULT_MANIFEST: &str = "crates/solosoul-vault/Cargo.toml";
const VAULT_SOURCE: &str = "crates/solosoul-vault/src/lib.rs";
const SIMPLE: &str =
    "#[tauri::command] pub fn read_objects()->Result<String,String>{unreachable!()}";

struct Fixture {
    root: TempDir,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fixture.write("src-tauri/src/lib.rs", "pub mod commands; fn dispatch(){tauri::generate_handler![commands::object::read_objects]}");
        fixture.write("src-tauri/src/commands/mod.rs", "pub mod object;");
        fixture.write(HOST, source);
        fixture.write(
            "src-tauri/permissions/solo-soul/default.toml",
            "[[permission]]\nidentifier='allow-app-commands'\ncommands.allow=['read_objects']\n",
        );
        fixture.selection(
            json!({"commands":[{"name":"read_objects","source":HOST}],"types":[],"events":[]}),
        );
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
    fn external(&self, source: &str) {
        self.write(
            "Cargo.toml",
            "[workspace]\nmembers=['src-tauri','crates/solosoul-vault']\n",
        );
        self.write("src-tauri/Cargo.toml", "[package]\nname='host'\nversion='0.1.0'\n[dependencies]\nsolosoul-vault={path='../crates/solosoul-vault'}\n");
        self.write(
            VAULT_MANIFEST,
            "[package]\nname='solosoul-vault'\nversion='0.1.0'\n",
        );
        self.write(VAULT_SOURCE, source);
        self.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":[VAULT_SOURCE],"events":[],"crates":[VAULT_MANIFEST]}));
    }
    fn generated(&self) -> Generated {
        generate(self.root.path()).unwrap()
    }
    fn error(&self) -> String {
        generate(self.root.path()).unwrap_err()
    }
}

#[test]
fn rf302_json_maps_and_vector_omission_follow_same_source_serde_directions() {
    let fixture=Fixture::new(&format!("{}\n#[tauri::command] pub fn read_objects(value:JsonMaps)->Result<JsonMaps,String>{{unreachable!()}}",include_str!("fixtures/rf302_dtos.rs")));
    let ts = fixture.generated().typescript;
    assert_eq!(ts.matches("export type JsonValue =").count(), 1);
    assert!(ts.contains(
        "export type JsonValue = null | boolean | number | string | Array<JsonValue> | JsonObject;"
    ));
    assert!(ts.contains("export type JsonObject = { [key: string]: JsonValue };"));
    let output = ts
        .lines()
        .find(|line| line.starts_with("export type JsonMaps ="))
        .unwrap();
    let input = ts
        .lines()
        .find(|line| line.starts_with("export type JsonMapsInput ="))
        .unwrap();
    assert!(output.contains("\"counts\": Record<string, number>;"));
    assert!(output.contains("\"nested\": Record<string, Array<(JsonValue) | null>>;"));
    assert!(output.contains("\"required_items\"?: Array<string>;"));
    assert!(input.contains("\"required_items\": Array<string>;"));
    assert!(input.contains("\"defaulted_items\"?: Array<string>;"));
    let mut value:host_fixture::JsonMaps=serde_json::from_value(json!({"payload":[null,true,7,"合成",{"x":[1,2]}],"counts":{"one":1},"nested":{"values":[null,{"a":false}]},"required_items":[]})).unwrap();
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serialized["payload"],
        json!([null,true,7,"合成",{"x":[1,2]}])
    );
    assert_eq!(serialized["nested"], json!({"values":[null,{"a":false}]}));
    assert!(
        serialized.get("required_items").is_none() && serialized.get("defaulted_items").is_none()
    );
    assert!(
        serde_json::from_value::<host_fixture::JsonMaps>(serialized).is_err(),
        "output omission cannot create an input default"
    );
    value.required_items.push("a".into());
    value.defaulted_items.push("b".into());
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(serialized["required_items"], json!(["a"]));
    assert_eq!(serialized["defaulted_items"], json!(["b"]));
}

#[test]
fn rf302_object_data_preserves_nullable_labels_and_optional_tags() {
    let fixture = Fixture::new(&format!(
        "{}\n#[tauri::command] pub fn read_objects()->Result<ObjectData,String>{{unreachable!()}}",
        include_str!("fixtures/rf302_dtos.rs")
    ));
    let ts = fixture.generated().typescript;
    assert!(ts.contains("\"propertyLabels\": (JsonValue) | null;"));
    assert!(ts.contains("\"tags\"?: Array<string>;"));
    assert!(ts.contains("\"properties\": JsonValue;"));
    let mut value:host_fixture::ObjectData=serde_json::from_value(json!({"id":"synthetic","accountId":"a","name":"合成对象","typeId":"note","properties":{"public":1,"nested":[null,true]},"sensitivityLevel":"internal","createdAt":"2026-01-01","updatedAt":"2026-01-02"})).unwrap();
    let empty = serde_json::to_value(&value).unwrap();
    assert_eq!(empty["propertyLabels"], serde_json::Value::Null);
    assert!(empty.get("propertyLabels").is_some());
    assert!(empty.get("templateId").is_some());
    assert!(empty.get("tags").is_none());
    value.tags.push("synthetic-tag".into());
    value.property_labels = Some(json!({"nested":"critical","unknown":"retain"}));
    let full = serde_json::to_value(value).unwrap();
    assert_eq!(full["tags"], json!(["synthetic-tag"]));
    assert_eq!(
        full["propertyLabels"],
        json!({"nested":"critical","unknown":"retain"})
    );
}

#[test]
fn rf302_external_summary_preserves_real_null_omission_and_field_names() {
    let fixture=Fixture::new("#[tauri::command] pub fn read_objects()->Result<Vec<solosoul_vault::ObjectSummary>,String>{unreachable!()}");
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    let generated = fixture.generated();
    let ts = &generated.typescript;
    assert!(ts.contains("result: Array<ObjectSummary>"));
    assert!(ts.contains("\"templateId\": (string) | null;"));
    assert!(ts.contains("\"contractTypeId\"?: string;"));
    assert!(ts.contains("\"propertyLabels\"?: JsonValue;"));
    assert!(ts.contains("\"tags\": Array<string>;"));
    assert!(ts.contains("\"sensitivityLevels\"?: Array<string>;"));
    assert!(ts.contains("\"hasAttachments\": boolean;"));
    let mut value:external_fixture::ObjectSummary=serde_json::from_value(json!({"id":"synthetic","name":"合成摘要","typeId":"note","sectionType":"custom","sensitivityLevel":"public","createdAt":"a","updatedAt":"b","isDeleted":false,"iconName":"file","properties":null,"tags":[]})).unwrap();
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(serialized["templateId"], serde_json::Value::Null);
    assert!(serialized.get("templateId").is_some());
    assert!(serialized.get("contractTypeId").is_none());
    assert!(serialized.get("propertyLabels").is_none());
    assert!(serialized.get("sensitivityLevels").is_none());
    assert_eq!(serialized["tags"], json!([]));
    assert_eq!(serialized["hasAttachments"], false);
    value.property_labels = Some(serde_json::Value::Null);
    value.sensitivity_levels = vec!["public".into(), "critical".into()];
    let populated = serde_json::to_value(value).unwrap();
    assert_eq!(
        populated["sensitivityLevels"],
        json!(["public", "critical"])
    );
    assert_eq!(
        populated.get("propertyLabels"),
        Some(&serde_json::Value::Null)
    );
    for source in [
        "Cargo.toml",
        "src-tauri/Cargo.toml",
        VAULT_MANIFEST,
        VAULT_SOURCE,
    ] {
        assert!(generated.manifest["sources"]
            .as_array()
            .unwrap()
            .contains(&json!(source)));
    }
    // 同时读取仓库真实 Vault AST，未来字段变化必须直接影响生成，而不是依赖复制的 wire 声明。
    fixture.write(
        VAULT_SOURCE,
        include_str!("../../../crates/solosoul-vault/src/lib.rs"),
    );
    assert_eq!(fixture.generated().typescript, *ts);
}

#[test]
fn rf302_external_crate_paths_resolve_against_their_own_root() {
    let fixture=Fixture::new("use solosoul_vault::CrossCrate; #[tauri::command] pub fn read_objects()->Result<CrossCrate,String>{unreachable!()}");
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    let generated = fixture.generated();
    assert!(generated
        .typescript
        .contains("export type CrossCrate = { \"nested\": SharedLeaf; };"));
    assert!(generated
        .typescript
        .contains("export type SharedLeaf = { \"value\": string; };"));
    assert_eq!(
        serde_json::to_value(external_fixture::CrossCrate {
            nested: SharedLeaf {
                value: "synthetic".into()
            }
        })
        .unwrap(),
        json!({"nested":{"value":"synthetic"}})
    );
    fixture.write(VAULT_SOURCE, "pub mod detail;");
    fixture.write(
        "crates/solosoul-vault/src/detail.rs",
        "#[derive(serde::Serialize)] pub struct CrossCrate{pub name:String}",
    );
    fixture.write(HOST,"#[tauri::command] pub fn read_objects()->Result<solosoul_vault::detail::CrossCrate,String>{unreachable!()}");
    fixture.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":["crates/solosoul-vault/src/detail.rs"],"events":[],"crates":[VAULT_MANIFEST]}));
    assert!(fixture.generated().typescript.contains("\"name\": string;"));
    fixture.write(VAULT_SOURCE, "");
    assert!(fixture.error().contains("exactly one declaration"));
}

#[test]
fn rf302_unregistered_or_forged_external_sources_are_rejected() {
    let source="#[tauri::command] pub fn read_objects()->Result<solosoul_vault::ObjectSummary,String>{unreachable!()}";
    let fixture = Fixture::new(source);
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    fixture.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":[VAULT_SOURCE],"events":[]}));
    assert!(fixture
        .error()
        .contains("outside a registered crate source root"));
    let corruptions=[
        ("Cargo.toml","[workspace]\nmembers=['src-tauri']\n"),
        ("src-tauri/Cargo.toml","[dependencies]\nsolosoul-vault={path='../crates/other'}\n"),
        ("src-tauri/Cargo.toml","[dependencies]\nsolosoul-vault={path='../crates/solosoul-vault',optional=true}\n"),
        ("src-tauri/Cargo.toml","[dependencies]\nsolosoul-vault={path='../crates/solosoul-vault',package='forged'}\n"),
        (VAULT_MANIFEST,"[package]\nname='forged'\n"),
        (VAULT_MANIFEST,"[package]\nname='solosoul-vault'\n[lib]\nname='forged'\n"),
        (VAULT_MANIFEST,"[package]\nname='solosoul-vault'\n[lib]\npath='../../src-tauri/src/commands/object.rs'\n"),
        (VAULT_MANIFEST,"[package]\nname='solosoul-vault'\nautolib=false\n"),
        (VAULT_MANIFEST,"[package]\nname='solosoul-vault'\n[lib]\nproc-macro=true\n"),
    ];
    for (path, contents) in corruptions {
        fixture.external(include_str!("fixtures/rf302_external.rs"));
        fixture.write(
            "crates/other/Cargo.toml",
            "[package]\nname='solosoul-vault'\n",
        );
        fixture.write(path, contents);
        assert!(
            generate(fixture.root.path()).is_err(),
            "accepted forged source: {path} {contents}"
        );
    }
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    fixture.write(HOST,"#[tauri::command] pub fn read_objects()->Result<crate::solosoul_vault::ObjectSummary,String>{unreachable!()}");
    assert!(fixture
        .error()
        .contains("crate:: cannot refer to an external dependency"));
    fixture.selection(json!({"commands":[{"name":"read_objects","source":VAULT_SOURCE}],"types":[VAULT_SOURCE],"events":[],"crates":[VAULT_MANIFEST]}));
    assert!(fixture.error().contains("does not match handler module"));
}

#[test]
fn rf302_external_manifest_inventory_is_deterministic_and_rejects_duplicates() {
    let fixture = Fixture::new(SIMPLE);
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    let first = fixture.generated();
    assert_eq!(first.typescript, fixture.generated().typescript);
    assert!(
        !first.typescript.contains("export type JsonValue"),
        "unreachable JSON DTOs must not introduce helpers"
    );
    fixture.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":[VAULT_SOURCE],"events":[],"crates":[VAULT_MANIFEST,VAULT_MANIFEST]}));
    assert!(fixture.error().contains("duplicate crate manifest"));
}

#[test]
fn rf302_json_and_hashmap_require_real_known_types_and_string_keys() {
    let fixture=Fixture::new("use serde_json::Value as Json; use std::collections::HashMap; #[tauri::command] pub fn read_objects(input:HashMap<String,Json>)->Result<Json,String>{unreachable!()}");
    assert!(fixture
        .generated()
        .typescript
        .contains("\"input\": Record<string, JsonValue>;"));
    for source in [
        "#[tauri::command] pub fn read_objects()->Result<std::collections::HashMap<u32,String>,String>{unreachable!()}",
        "#[tauri::command] pub fn read_objects()->Result<std::collections::HashMap<String,String,CustomHasher>,String>{unreachable!()}",
        "#[tauri::command] pub fn read_objects()->Result<HashMap<String,String>,String>{unreachable!()}",
        "use other_json::Value; #[tauri::command] pub fn read_objects()->Result<Value,String>{unreachable!()}",
        "#[derive(serde::Serialize)] pub struct JsonValue{pub x:String} #[tauri::command] pub fn read_objects()->Result<JsonValue,String>{unreachable!()}",
        "#[derive(serde::Serialize)] pub struct JsonObject{pub x:String} #[tauri::command] pub fn read_objects()->Result<JsonObject,String>{unreachable!()}",
    ] { fixture.write(HOST,source); assert!(generate(fixture.root.path()).is_err(),"unsupported type accepted: {source}"); }
}

#[test]
fn rf302_vec_skip_predicate_does_not_accept_wrong_type_or_unknown_predicate() {
    for field in [
        "#[serde(skip_serializing_if=\"Vec::is_empty\")] pub field:String",
        "#[serde(skip_serializing_if=\"Vec::is_empty\")] pub field:Option<Vec<String>>",
        "#[serde(skip_serializing_if=\"custom::is_empty\")] pub field:Vec<String>",
    ] {
        let fixture=Fixture::new(&format!("#[derive(serde::Serialize)] pub struct Data{{{field}}} #[tauri::command] pub fn read_objects()->Result<Data,String>{{unreachable!()}}"));
        assert!(
            generate(fixture.root.path()).is_err(),
            "unsupported predicate accepted: {field}"
        );
    }
}

#[test]
fn rf302_explicit_library_path_and_namespace_collisions_are_checked() {
    let fixture=Fixture::new("#[tauri::command] pub fn read_objects()->Result<solosoul_vault::ObjectSummary,String>{unreachable!()}");
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    fixture.write(
        VAULT_MANIFEST,
        "[package]\nname='solosoul-vault'\n[lib]\nname='solosoul_vault'\npath='wire/root.rs'\n",
    );
    fixture.write(
        "crates/solosoul-vault/wire/root.rs",
        include_str!("fixtures/rf302_external.rs"),
    );
    fixture.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":["crates/solosoul-vault/wire/root.rs"],"events":[],"crates":[VAULT_MANIFEST]}));
    assert!(fixture
        .generated()
        .typescript
        .contains("export type ObjectSummary ="));
    fixture.selection(json!({"commands":[{"name":"read_objects","source":HOST}],"types":[VAULT_SOURCE],"events":[],"crates":[VAULT_MANIFEST]}));
    assert!(
        fixture
            .error()
            .contains("outside a registered crate source root"),
        "old src/lib.rs must not masquerade as configured lib.path"
    );
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    fixture.write("src-tauri/src/lib.rs","pub mod commands; pub mod solosoul_vault; fn dispatch(){tauri::generate_handler![commands::object::read_objects]}");
    assert!(fixture.error().contains("collides with Host module"));
}

#[test]
fn rf302_command_local_module_cannot_masquerade_as_registered_external_crate() {
    for declaration in [
        "mod solosoul_vault { #[derive(serde::Serialize)] pub struct ObjectSummary { pub changed: bool } }",
        "mod solosoul_vault;",
        "mod r#solosoul_vault { #[derive(serde::Serialize)] pub struct ObjectSummary { pub changed: bool } }",
        "#[cfg(any())] mod solosoul_vault { pub struct ObjectSummary; }",
        "extern crate other as solosoul_vault;",
    ] {
        let fixture=Fixture::new(&format!("{declaration} #[tauri::command] pub fn read_objects()->Result<solosoul_vault::ObjectSummary,String>{{unreachable!()}}"));
        fixture.external(include_str!("fixtures/rf302_external.rs"));
        fixture.write("src-tauri/src/commands/object/solosoul_vault.rs", "#[derive(serde::Serialize)] pub struct ObjectSummary{pub changed:bool}");
        assert!(fixture.error().contains("local binding solosoul_vault shadows known contract namespace"),"accepted local binding: {declaration}");
    }
}

#[test]
fn rf302_local_modules_cannot_masquerade_as_builtin_json_map_or_tauri_types() {
    let cases=[
        ("serde_json", "mod serde_json{#[derive(serde::Serialize)] pub struct Value{pub changed:bool}} #[tauri::command] pub fn read_objects()->Result<serde_json::Value,String>{unreachable!()}"),
        ("std", "mod std{pub mod collections{#[derive(serde::Serialize)] pub struct HashMap<K,V>(K,V);}} #[tauri::command] pub fn read_objects()->Result<std::collections::HashMap<String,String>,String>{unreachable!()}"),
        ("tauri", "mod tauri{pub use ::tauri::command; #[derive(serde::Deserialize)] pub struct State<T>(T);} #[tauri::command] pub fn read_objects(state:tauri::State<String>)->Result<String,String>{unreachable!()}"),
        ("serde", "mod serde{} #[derive(serde::Serialize)] pub struct Data{pub changed:bool} #[tauri::command] pub fn read_objects()->Result<Data,String>{unreachable!()}"),
        ("core", "mod core{pub mod option{#[derive(serde::Serialize)] pub struct Option<T>(T);}} #[tauri::command] pub fn read_objects()->Result<core::option::Option<String>,String>{unreachable!()}"),
        ("alloc", "mod alloc{pub mod vec{#[derive(serde::Serialize)] pub struct Vec<T>(T);}} #[tauri::command] pub fn read_objects()->Result<alloc::vec::Vec<String>,String>{unreachable!()}"),
    ];
    for (name, source) in cases {
        let fixture = Fixture::new(source);
        assert!(
            fixture.error().contains(&format!(
                "local binding {name} shadows known contract namespace"
            )),
            "accepted local builtin namespace: {name}"
        );
    }
}

#[test]
fn rf302_external_dto_source_and_module_ancestors_reject_namespace_shadowing() {
    let fixture=Fixture::new("#[tauri::command] pub fn read_objects()->Result<solosoul_vault::Data,String>{unreachable!()}");
    fixture.external("mod serde_json{#[derive(serde::Serialize)] pub struct Value{pub changed:bool}} #[derive(serde::Serialize)] pub struct Data{pub value:serde_json::Value}");
    assert!(fixture
        .error()
        .contains("local binding serde_json shadows known contract namespace"));
    fixture.external(include_str!("fixtures/rf302_external.rs"));
    fixture.write(HOST, SIMPLE);
    fixture.write(
        "src-tauri/src/commands/mod.rs",
        "pub mod object; mod solosoul_vault{}",
    );
    assert!(fixture
        .error()
        .contains("local binding solosoul_vault shadows known contract namespace"));
    fixture.write("src-tauri/src/commands/mod.rs", "pub mod object;");
    fixture.write("src-tauri/src/lib.rs","pub mod commands; mod serde_json{} fn dispatch(){tauri::generate_handler![commands::object::read_objects]}");
    assert!(fixture
        .error()
        .contains("local binding serde_json shadows known contract namespace"));
}
