//! RF-1063：真实加密包、生产 owned job 与 SQLite 验证 KeepBoth 的统一映射。
//! 不替换导入、映射或附件实现；仅复用已存在的临时 Vault / ZIP 帮助函数。
use super::super::import::{run_import_job, ImportJob};
use super::rf020::{objects, package, Fixture};
use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IDS: [&str; 2] = ["rf020-0", "rf020-1"];

fn request(path: &Path, strategy: ImportStrategy) -> AdvancedImportRequest {
    AdvancedImportRequest {
        selections: None,
        strategy,
        source_path: path.to_str().unwrap().into(),
        password: "export-password".into(),
        selected_attachment_ids: Some(vec![]),
        object_strategies: HashMap::new(),
        locale: "en-US".into(),
    }
}

fn run(f: &Fixture, req: AdvancedImportRequest) -> ImportResult {
    let path = PathBuf::from(&req.source_path);
    let original = std::fs::read(&path).unwrap();
    let job = ImportJob::prepare(f.service.clone(), &f.account, req, None, |path| {
        Path::new(path).canonicalize().map_err(|e| e.to_string())
    })
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime
        .block_on(run_import_job(job, ImportJob::run, || {}))
        .unwrap();
    assert!(result.is_complete(), "{result:?}");
    assert_eq!(std::fs::read(path).unwrap(), original);
    result
}

fn graph_payload() -> serde_json::Value {
    let mut payload = objects();
    let values = payload["objects"].as_array_mut().unwrap();
    values[0]["name"] = json!("Parent");
    values[0]["children_ids"] = json!([IDS[1]]);
    values[0]["properties"]["marker"] = json!("parent");
    values[0]["properties"]["nested"] = json!([
        {"type":"relation", "targetId": IDS[1]},
        {"__type":"relation", "id": IDS[1]},
        {"kind":"relation", "objectId": IDS[1]}
    ]);
    values[1]["name"] = json!("Child");
    values[1]["parent_id"] = json!(IDS[0]);
    values[1]["properties"]["marker"] = json!("child");
    values[1]["properties"]["parent"] = json!({"type":"relation", "targetId": IDS[0]});
    payload
}

fn records(f: &Fixture) -> Vec<ObjectRecord> {
    f.vault
        .list_objects(&f.account, None, None, None, true, false)
        .unwrap()
        .into_iter()
        .map(|summary| f.vault.load_object(&summary.id).unwrap().unwrap())
        .collect()
}

fn marked(f: &Fixture, marker: &str) -> ObjectRecord {
    let matching: Vec<_> = records(f)
        .into_iter()
        .filter(|record| record.properties["marker"] == marker)
        .collect();
    assert_eq!(matching.len(), 1, "marker={marker}");
    matching.into_iter().next().unwrap()
}

fn assert_forward_links(parent: &ObjectRecord, child_id: &str) {
    assert_eq!(parent.children_ids, vec![child_id.to_string()]);
    assert_eq!(parent.properties["nested"][0]["targetId"], child_id);
    assert_eq!(parent.properties["nested"][1]["id"], child_id);
    assert_eq!(parent.properties["nested"][2]["objectId"], child_id);
}

fn save_local(f: &Fixture, id: &str, name: &str) -> ObjectRecord {
    let record = ObjectRecord {
        id: id.into(),
        account_id: f.account.clone(),
        type_id: "note".into(),
        section_type: "identity".into(),
        name: name.into(),
        properties: json!({"local": name}),
        created_at: "2026-09-30T00:00:00Z".into(),
        updated_at: "2026-09-30T00:00:00Z".into(),
        version: 7,
        ..Default::default()
    };
    f.vault.save_object(&record).unwrap();
    f.vault.load_object(id).unwrap().unwrap()
}

fn physical_files(f: &Fixture) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, files);
            } else {
                files.insert(path.clone(), std::fs::read(path).unwrap());
            }
        }
    }
    let root = f.service.read().unwrap().base_path().join("attachments");
    let mut files = BTreeMap::new();
    visit(&root, &mut files);
    files
}

fn seed_graph(f: &Fixture, path: &Path) {
    let mut req = request(path, ImportStrategy::Overwrite);
    req.selected_attachment_ids = None;
    let result = run(f, req);
    assert_eq!((result.object_count, result.attachment_count), (2, 2));
}

#[test]
fn rf1063_global_keepboth_empty_overrides_rewrites_graph_and_selected_attachment() {
    let f = Fixture::new();
    let path = package(f.dir.path(), graph_payload(), true, false, false);
    let mut req = request(&path, ImportStrategy::KeepBoth);
    assert!(req.object_strategies.is_empty()); // 与 UI 选择全局 KeepBoth 时发送的形状一致。
    req.selected_attachment_ids = Some(vec!["a0".into()]);
    let result = run(&f, req);
    assert_eq!((result.object_count, result.attachment_count), (2, 1));
    assert_eq!(result.attachment_files_written, 1);
    let parent = marked(&f, "parent");
    let child = marked(&f, "child");
    assert_ne!(parent.id, IDS[0]);
    assert_ne!(child.id, IDS[1]);
    assert_forward_links(&parent, &child.id);
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
    assert_eq!(child.properties["parent"]["targetId"], parent.id);
    assert_eq!(parent.name, "Parent (Imported)");
    assert_eq!(child.name, "Child (Imported)");
    assert!(f.vault.load_object(IDS[0]).unwrap().is_none());
    assert!(f.vault.load_object(IDS[1]).unwrap().is_none());
    let atts = parent.properties["__attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 1);
    assert_eq!(atts[0]["objectId"], parent.id);
    assert_ne!(atts[0]["id"], "a0");
    let file = PathBuf::from(atts[0]["vaultPath"].as_str().unwrap());
    let base = f.service.read().unwrap().base_path().to_path_buf();
    assert!(file.starts_with(base.join("attachments").join(&parent.id)));
    assert!(solosoul_core::attachment_crypto::is_encrypted_file(&file));
    let service = f.service.read().unwrap();
    let session = service.capture_session(&f.account).unwrap();
    let key = service.attachment_key_for_session(&session).unwrap();
    assert_eq!(
        solosoul_core::attachment_crypto::read_file_decrypted(&key, &file, 1024).unwrap(),
        b"fixture"
    );
    drop(service);
    assert_eq!(physical_files(&f).len(), 1);
}

#[test]
fn rf1063_nonkeepboth_override_of_global_keepboth_retains_original_id() {
    for override_strategy in [ImportStrategy::Overwrite, ImportStrategy::SkipExisting] {
        let f = Fixture::new();
        let local = save_local(&f, IDS[0], "Local parent");
        let path = package(f.dir.path(), graph_payload(), false, false, false);
        let mut req = request(&path, ImportStrategy::KeepBoth);
        req.object_strategies
            .insert(IDS[0].into(), override_strategy);
        let result = run(&f, req);
        let parent = f.vault.load_object(IDS[0]).unwrap().unwrap();
        let child = marked(&f, "child");
        assert_ne!(child.id, IDS[1]);
        assert_eq!(child.parent_id.as_deref(), Some(IDS[0]));
        assert_eq!(child.properties["parent"]["targetId"], IDS[0]);
        if override_strategy == ImportStrategy::Overwrite {
            assert_eq!(result.object_count, 2);
            assert_eq!(parent.name, "Parent");
            assert_forward_links(&parent, &child.id);
        } else {
            assert_eq!(result.object_count, 1);
            assert_eq!(
                serde_json::to_value(parent).unwrap(),
                serde_json::to_value(local).unwrap()
            );
        }
    }
}

#[test]
fn rf1063_per_object_keepboth_with_nonkeepboth_global_still_remaps() {
    for strategy in [ImportStrategy::Overwrite, ImportStrategy::SkipExisting] {
        let f = Fixture::new();
        let local = save_local(&f, IDS[0], "Local parent");
        let path = package(f.dir.path(), graph_payload(), false, false, false);
        let mut req = request(&path, strategy);
        req.object_strategies
            .insert(IDS[1].into(), ImportStrategy::KeepBoth);
        let result = run(&f, req);
        let parent = f.vault.load_object(IDS[0]).unwrap().unwrap();
        let child = marked(&f, "child");
        assert_ne!(child.id, IDS[1]);
        assert_eq!(child.parent_id.as_deref(), Some(IDS[0]));
        assert_eq!(child.properties["parent"]["targetId"], IDS[0]);
        if strategy == ImportStrategy::Overwrite {
            assert_eq!(result.object_count, 2);
            assert_forward_links(&parent, &child.id);
        } else {
            assert_eq!(result.object_count, 1);
            assert_eq!(
                serde_json::to_value(parent).unwrap(),
                serde_json::to_value(local).unwrap()
            );
        }
    }
}

fn assert_unselected_target_is_unchanged(global_keepboth: bool) {
    let f = Fixture::new();
    let path = package(f.dir.path(), graph_payload(), true, false, false);
    seed_graph(&f, &path);
    let old_parent = f.vault.load_object(IDS[0]).unwrap().unwrap();
    let old_child = f.vault.load_object(IDS[1]).unwrap().unwrap();
    let old_child_json = serde_json::to_value(&old_child).unwrap();
    let before_files = physical_files(&f);
    let before_child_snapshots = f.vault.list_snapshots(IDS[1]).unwrap();
    let mut changed_payload = graph_payload();
    changed_payload["objects"][1]["name"] = json!("Do not import this child");
    changed_payload["objects"][1]["properties"]["marker"] = json!("unselected replacement");
    let path = package(f.dir.path(), changed_payload, true, false, false);
    let mut req = request(
        &path,
        if global_keepboth {
            ImportStrategy::KeepBoth
        } else {
            ImportStrategy::Overwrite
        },
    );
    req.selections = Some(vec![
        ImportSelection {
            object_id: IDS[0].into(),
            selected: true,
        },
        ImportSelection {
            object_id: IDS[1].into(),
            selected: false,
        },
    ]);
    req.selected_attachment_ids = Some(vec!["a0".into(), "a1".into()]);
    if !global_keepboth {
        // 原错误会给未选中的显式 KeepBoth 对象也建映射，令 A 指向不存在的 B 新 ID。
        req.object_strategies
            .insert(IDS[0].into(), ImportStrategy::KeepBoth);
        req.object_strategies
            .insert(IDS[1].into(), ImportStrategy::KeepBoth);
    }
    let result = run(&f, req);
    assert_eq!((result.object_count, result.attachment_count), (1, 1));
    let copies: Vec<_> = records(&f)
        .into_iter()
        .filter(|record| record.id != IDS[0] && record.id != IDS[1])
        .collect();
    assert_eq!(copies.len(), 1);
    let copy = &copies[0];
    assert_forward_links(copy, IDS[1]);
    assert_eq!(copy.name, "Parent (Imported)");
    assert_eq!(copy.properties["__attachments"][0]["objectId"], copy.id);
    assert_eq!(
        serde_json::to_value(f.vault.load_object(IDS[0]).unwrap().unwrap()).unwrap(),
        serde_json::to_value(old_parent).unwrap()
    );
    assert_eq!(
        serde_json::to_value(f.vault.load_object(IDS[1]).unwrap().unwrap()).unwrap(),
        old_child_json
    );
    assert_eq!(
        f.vault.list_snapshots(IDS[1]).unwrap(),
        before_child_snapshots
    );
    let after_files = physical_files(&f);
    assert_eq!(after_files.len(), before_files.len() + 1);
    for (path, bytes) in before_files {
        assert_eq!(after_files.get(&path), Some(&bytes));
    }
    // 不仅不新增 B，也不改旧 B 的附件目录或元数据；仅 A 的副本多出一个实体附件。
    assert_eq!(
        copies
            .iter()
            .filter(|record| record.properties["marker"] == "unselected replacement")
            .count(),
        0
    );
}

#[test]
fn rf1063_global_keepboth_unselected_target_keeps_original_reference_and_attachments() {
    assert_unselected_target_is_unchanged(true);
}

#[test]
fn rf1063_explicit_keepboth_unselected_target_keeps_original_reference_and_attachments() {
    assert_unselected_target_is_unchanged(false);
}

#[test]
fn rf1063_empty_selection_does_not_create_copies_or_modify_existing_data() {
    for global_keepboth in [true, false] {
        let f = Fixture::new();
        let path = package(f.dir.path(), graph_payload(), true, false, false);
        seed_graph(&f, &path);
        let before_records: Vec<_> = IDS
            .into_iter()
            .map(|id| serde_json::to_value(f.vault.load_object(id).unwrap().unwrap()).unwrap())
            .collect();
        let before_files = physical_files(&f);
        let before_snapshots: Vec<_> = IDS
            .into_iter()
            .map(|id| f.vault.list_snapshots(id).unwrap())
            .collect();
        let mut req = request(
            &path,
            if global_keepboth {
                ImportStrategy::KeepBoth
            } else {
                ImportStrategy::Overwrite
            },
        );
        req.selections = Some(vec![]);
        req.selected_attachment_ids = None;
        if !global_keepboth {
            for id in IDS {
                req.object_strategies
                    .insert(id.into(), ImportStrategy::KeepBoth);
            }
        }
        let result = run(&f, req);
        assert_eq!(
            (
                result.object_count,
                result.snapshot_count,
                result.attachment_count,
                result.attachment_files_written
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(records(&f).len(), 2);
        assert_eq!(physical_files(&f), before_files);
        for (index, id) in IDS.into_iter().enumerate() {
            assert_eq!(
                serde_json::to_value(f.vault.load_object(id).unwrap().unwrap()).unwrap(),
                before_records[index]
            );
            assert_eq!(f.vault.list_snapshots(id).unwrap(), before_snapshots[index]);
        }
    }
}

#[test]
fn rf1063_duplicate_selected_source_ids_share_final_mapping_and_original_name_order() {
    for global_keepboth in [true, false] {
        let f = Fixture::new();
        let mut first = graph_payload()["objects"][0].clone();
        first["name"] = json!("Duplicate");
        first["children_ids"] = json!([IDS[0]]);
        first["properties"] = json!({"ordinal":1, "self":{"type":"relation", "targetId":IDS[0]}});
        let mut second = first.clone();
        second["properties"]["ordinal"] = json!(2);
        let path = package(
            f.dir.path(),
            json!({"objects":[first, second]}),
            false,
            false,
            false,
        );
        let mut req = request(
            &path,
            if global_keepboth {
                ImportStrategy::KeepBoth
            } else {
                ImportStrategy::Overwrite
            },
        );
        req.selections = Some(vec![ImportSelection {
            object_id: IDS[0].into(),
            selected: true,
        }]);
        if !global_keepboth {
            req.object_strategies
                .insert(IDS[0].into(), ImportStrategy::KeepBoth);
        }
        let result = run(&f, req);
        assert_eq!((result.object_count, result.snapshot_count), (1, 2));
        let imported = records(&f);
        assert_eq!(imported.len(), 1);
        let last = &imported[0];
        assert_ne!(last.id, IDS[0]);
        assert_eq!(last.properties["ordinal"], 2);
        assert_eq!(last.properties["self"]["targetId"], last.id);
        assert_eq!(last.children_ids, vec![last.id.clone()]);
        assert_eq!(last.name, "Duplicate (Imported) 2");
        assert_eq!(f.vault.list_snapshots(&last.id).unwrap().len(), 2);
        assert!(f.vault.load_object(IDS[0]).unwrap().is_none());
    }
}

#[test]
fn rf1063_global_keepboth_keeps_locale_suffix_and_name_increment_rules() {
    for (locale, suffix) in [("en-US", " (Imported)"), ("zh-CN", "（导入）")] {
        let f = Fixture::new();
        let mut payload = graph_payload();
        for value in payload["objects"].as_array_mut().unwrap() {
            value["name"] = json!("Same");
        }
        let path = package(f.dir.path(), payload, false, false, false);
        let mut req = request(&path, ImportStrategy::KeepBoth);
        req.locale = locale.into();
        let result = run(&f, req);
        assert_eq!(result.object_count, 2);
        assert_eq!(marked(&f, "parent").name, format!("Same{suffix}"));
        assert_eq!(marked(&f, "child").name, format!("Same{suffix} 2"));
    }
}
