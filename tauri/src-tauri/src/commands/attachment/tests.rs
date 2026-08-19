//! 附件命令单元测试（P047 拆分：mod.rs 中 `#[cfg(test)] mod tests;` 指向本文件，
//! 文件根即 attachment::tests 模块，不再重复包裹 `mod tests`）。

use super::*;
// P047：父模块 use 绑定不随 `use super::*` glob 导入，Path 需显式引入
use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
// P047：tree/share 子模块的 pub(crate) 项需显式导入（mod.rs 不做 re-export，避免非测试构建 unused 警告）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use super::share::{cleanup_share_dir, copy_into_dir};
use super::tree::{build_attachment_tree_pages, group_objects_for_attachment_tree};

fn setup_vault() -> (VaultStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let config =
        VaultConfig::new("test_account", dir.path().to_path_buf()).with_data_key([0x42u8; 32]);
    let vault = VaultStore::open(config).unwrap();
    (vault, dir)
}

/// R2-X1: 路径判定纯函数防回归测试（symlink 旁路 / Android 双路径 / 边界）。
#[test]
fn test_path_within_base_canonical_only() {
    // canonicalize 成功：只用 resolved 判定，字面路径共享前缀不能绕过。
    let base_canon = Path::new("/vault");
    let base_raw = Path::new("/vault");
    // resolved 在 base 内 → true
    assert!(path_within_base(
        Path::new("/vault/attachments/a.bin"),
        Path::new("/vault/attachments/a.bin"),
        true,
        base_canon,
        base_raw,
    ));
    // resolved 在 base 外，但 raw 字面前缀命中 → 仍 false（旁路封死）
    assert!(!path_within_base(
        Path::new("/real_outside/a.bin"),
        Path::new("/vault/attachments/a.bin"),
        true,
        base_canon,
        base_raw,
    ));
    // 完全无关 → false
    assert!(!path_within_base(
        Path::new("/etc/passwd"),
        Path::new("/etc/passwd"),
        true,
        base_canon,
        base_raw,
    ));
}

#[test]
fn test_path_within_base_raw_fallback_canonical() {
    // canonicalize 失败（Android 兜底）：raw 命中 canonical base → true
    let base_canon = Path::new("/vault");
    let base_raw = Path::new("/vault");
    assert!(path_within_base(
        Path::new("/vault/attachments/a.bin"),
        Path::new("/vault/attachments/a.bin"),
        false,
        base_canon,
        base_raw,
    ));
    // 与 base 完全无关 → false
    assert!(!path_within_base(
        Path::new("/etc/passwd"),
        Path::new("/etc/passwd"),
        false,
        base_canon,
        base_raw,
    ));
}

#[test]
fn test_path_within_base_rejects_parent_dir_escape() {
    // P018：兜底分支拒绝含 `..` 的逃逸路径（前几段命中 base 但实际越出）
    let base_canon = Path::new("/vault");
    let base_raw = Path::new("/vault");
    for bad in [
        "/vault/../../etc/passwd",
        "/vault/attachments/../..//etc/passwd",
        "/vault/../vault_evil/secret",
    ] {
        assert!(
            !path_within_base(Path::new(bad), Path::new(bad), false, base_canon, base_raw,),
            "should reject parent-dir escape: {bad}"
        );
    }
    // 无 `..` 的正常库内路径仍放行
    assert!(path_within_base(
        Path::new("/vault/attachments/a.bin"),
        Path::new("/vault/attachments/a.bin"),
        false,
        base_canon,
        base_raw,
    ));
}

#[test]
fn test_path_within_base_raw_fallback_dual_path() {
    // Android 双路径：raw 前缀是 /data/data（非 canonical），canonical base 是
    // /data/user/0——raw 仅命中 base_raw 时也应判定为库内（copy_to_vault 拒绝型）。
    let base_canon = Path::new("/data/user/0/com.solosoul");
    let base_raw = Path::new("/data/data/com.solosoul");
    // raw 命中 base_raw → true（此前 a||a 恒等会漏检）
    assert!(path_within_base(
        Path::new("/data/user/0/com.solosoul/attachments/a.bin"),
        Path::new("/data/data/com.solosoul/attachments/a.bin"),
        false,
        base_canon,
        base_raw,
    ));
    // 与 base 完全无关 → false
    assert!(!path_within_base(
        Path::new("/storage/emulated/0/Download/x.bin"),
        Path::new("/storage/emulated/0/Download/x.bin"),
        false,
        base_canon,
        base_raw,
    ));
    // canonicalize 成功时双路径也覆盖（resolved 命中 canonical）
    assert!(path_within_base(
        Path::new("/data/user/0/com.solosoul/attachments/a.bin"),
        Path::new("/data/data/com.solosoul/attachments/a.bin"),
        true,
        base_canon,
        base_raw,
    ));
}

#[test]
fn test_attachment_meta_serde_roundtrip() {
    let original = AttachmentMeta {
        id: "att-1".to_string(),
        object_id: "obj-1".to_string(),
        file_name: "test.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        size_bytes: 1024,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
        src_path: Some("/tmp/test.pdf".to_string()),
        vault_path: Some("/vault/test.pdf".to_string()),
        description: Some("a test attachment".to_string()),
        tags: vec!["scanned".to_string(), "receipt".to_string()],
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: AttachmentMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.object_id, original.object_id);
    assert_eq!(restored.file_name, original.file_name);
    assert_eq!(restored.mime_type, original.mime_type);
    assert_eq!(restored.size_bytes, original.size_bytes);
    assert_eq!(restored.created_at, original.created_at);
    assert_eq!(restored.deleted_at, original.deleted_at);
    assert_eq!(restored.src_path, original.src_path);
    assert_eq!(restored.vault_path, original.vault_path);
}

#[test]
fn test_load_attachments_empty() {
    let props = serde_json::json!({"title": "hello"});
    let atts = load_attachments(&props);
    assert!(atts.is_empty());
}

#[test]
fn test_load_attachments_some() {
    let atts = vec![AttachmentMeta {
        id: "att-1".to_string(),
        object_id: "obj-1".to_string(),
        file_name: "a.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        size_bytes: 100,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: None,
        src_path: None,
        vault_path: None,
        description: None,
        tags: vec![],
    }];
    let props = serde_json::json!({"title": "hello", "__attachments": atts});
    let loaded = load_attachments(&props);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "att-1");
}

#[test]
fn test_save_and_load_attachments() {
    let mut props = serde_json::json!({"title": "hello"});
    let atts = vec![AttachmentMeta {
        id: "att-1".to_string(),
        object_id: "obj-1".to_string(),
        file_name: "a.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        size_bytes: 100,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: None,
        src_path: None,
        vault_path: None,
        description: None,
        tags: vec![],
    }];
    save_attachments(&mut props, &atts);
    let loaded = load_attachments(&props);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "att-1");
    assert_eq!(loaded[0].file_name, "a.pdf");
}

#[test]
fn test_load_all_referenced_attachment_ids() {
    let (vault, _dir) = setup_vault();
    let account_id = "acc-1";

    let record1 = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Note 1".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__attachments": [
                AttachmentMeta {
                    id: "att-1".to_string(),
                    object_id: "obj-1".to_string(),
                    file_name: "a.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    size_bytes: 100,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    deleted_at: None,
                    src_path: None,
                    vault_path: None,
                    description: None,
                    tags: vec![],
                },
                AttachmentMeta {
                    id: "att-2".to_string(),
                    object_id: "obj-1".to_string(),
                    file_name: "b.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    size_bytes: 200,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                    src_path: None,
                    vault_path: None,
                    description: None,
                    tags: vec![],
                },
            ]
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record1).unwrap();

    let record2 = ObjectRecord {
        contract_type_id: None,
        id: "obj-2".to_string(),
        account_id: account_id.to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Note 2".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__attachments": [
                AttachmentMeta {
                    id: "att-3".to_string(),
                    object_id: "obj-2".to_string(),
                    file_name: "c.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    size_bytes: 300,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    deleted_at: None,
                    src_path: None,
                    vault_path: None,
                    description: None,
                    tags: vec![],
                },
            ]
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record2).unwrap();

    let ids = load_all_referenced_attachment_ids(&vault, account_id).unwrap();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains("att-1"));
    assert!(ids.contains("att-2"));
    assert!(ids.contains("att-3"));
}

#[test]
fn test_vault_attachment_filtering() {
    let (vault, _dir) = setup_vault();
    let mut record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "Note".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({
            "__attachments": [
                AttachmentMeta {
                    id: "att-1".to_string(),
                    object_id: "obj-1".to_string(),
                    file_name: "active.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    size_bytes: 100,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    deleted_at: None,
                    src_path: None,
                    vault_path: None,
                    description: None,
                    tags: vec![],
                },
                AttachmentMeta {
                    id: "att-2".to_string(),
                    object_id: "obj-1".to_string(),
                    file_name: "deleted.pdf".to_string(),
                    mime_type: "application/pdf".to_string(),
                    size_bytes: 200,
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    deleted_at: Some("2024-02-01T00:00:00Z".to_string()),
                    src_path: None,
                    vault_path: None,
                    description: None,
                    tags: vec![],
                },
            ]
        }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        version: 1,
        ..Default::default()
    };
    vault.save_object(&record).unwrap();

    let rec = vault.load_object("obj-1").unwrap().unwrap();
    let atts = load_attachments(&rec.properties);
    let active: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_none()).collect();
    let deleted: Vec<_> = atts.iter().filter(|a| a.deleted_at.is_some()).collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, "att-1");
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, "att-2");

    // Test soft-delete helper logic inline
    let mut atts_mut = load_attachments(&rec.properties);
    if let Some(a) = atts_mut.iter_mut().find(|a| a.id == "att-1") {
        a.deleted_at = Some("2024-03-01T00:00:00Z".to_string());
    }
    save_attachments(&mut record.properties, &atts_mut);
    vault.save_object(&record).unwrap();

    let rec2 = vault.load_object("obj-1").unwrap().unwrap();
    let atts2 = load_attachments(&rec2.properties);
    assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_none()).count(), 0);
    assert_eq!(atts2.iter().filter(|a| a.deleted_at.is_some()).count(), 2);
}

/// P112 回归：`attachment_list_all` 数据流不再重复解密——子对象按 parent_id 预分组
/// 一次完成（替代每页面 N+1 次解密查询），且 `build_attachment_tree_pages` 直接复用
/// 已解密的 `summary.properties`（不再 load_objects_batch 二次全量解密）。
/// 覆盖：页面含子对象附件、无附件对象不出现在树中、独立对象按 section 分组、
/// 回收站视图只含已删除附件、分组 map 幂等（活动视图与回收站视图共享同一分组）。
#[test]
fn test_attachment_list_all_groups_children_and_reuses_summary_properties() {
    let (vault, _dir) = setup_vault();
    let account_id = "acc-1";

    let mk_meta = |id: &str, obj_id: &str, deleted: bool| AttachmentMeta {
        id: id.to_string(),
        object_id: obj_id.to_string(),
        file_name: format!("{}.pdf", id),
        mime_type: "application/pdf".to_string(),
        size_bytes: 100,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: deleted.then(|| "2024-02-01T00:00:00Z".to_string()),
        src_path: None,
        vault_path: None,
        description: None,
        tags: vec![],
    };
    let mk_record = |id: &str,
                     type_id: &str,
                     section_type: &str,
                     parent: Option<&str>,
                     atts: Vec<AttachmentMeta>|
     -> ObjectRecord {
        ObjectRecord {
            contract_type_id: None,
            id: id.to_string(),
            account_id: account_id.to_string(),
            type_id: type_id.to_string(),
            section_type: section_type.to_string(),
            name: id.to_string(),
            icon_name: "document".to_string(),
            parent_id: parent.map(String::from),
            children_ids: vec![],
            properties: serde_json::json!({ "__attachments": atts }),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            ignored_template_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            version: 1,
        }
    };

    // 自定义页面 + 子对象（带附件）
    vault
        .save_object(&mk_record("page-1", "page", "", None, vec![]))
        .unwrap();
    vault
        .save_object(&mk_record(
            "obj-child",
            "note",
            "custom",
            Some("page-1"),
            vec![mk_meta("att-child", "obj-child", false)],
        ))
        .unwrap();
    // 独立对象（内置 section，带一个活动附件 + 一个已删除附件）
    vault
        .save_object(&mk_record(
            "obj-standalone",
            "note",
            "identity",
            None,
            vec![
                mk_meta("att-act", "obj-standalone", false),
                mk_meta("att-del", "obj-standalone", true),
            ],
        ))
        .unwrap();
    // 无附件的独立对象（不应出现在树中）
    vault
        .save_object(&mk_record("obj-empty", "note", "identity", None, vec![]))
        .unwrap();

    let objects = vault
        .list_objects(account_id, None, None, None, false, false)
        .unwrap();
    let (page_objects, section_groups, children_by_parent) =
        group_objects_for_attachment_tree(&objects);

    // 子对象按 parent_id 一次性分组（每页面无需再查）
    assert_eq!(children_by_parent.len(), 1);
    let child_summaries = children_by_parent.get("page-1").unwrap();
    assert_eq!(child_summaries.len(), 1);
    assert_eq!(child_summaries[0].id, "obj-child");

    // 活动视图：页面树含子对象附件，section 树含独立对象活动附件（无附件对象被过滤）
    let pages = build_attachment_tree_pages(
        &vault,
        &page_objects,
        &section_groups,
        &children_by_parent,
        false,
    )
    .unwrap();
    let page_tree = pages
        .iter()
        .find(|p| p.page_id.as_deref() == Some("page-1"))
        .expect("page-1 tree exists");
    assert_eq!(page_tree.objects.len(), 1);
    assert_eq!(page_tree.objects[0].object_id, "obj-child");
    assert_eq!(page_tree.objects[0].attachments.len(), 1);
    assert_eq!(page_tree.objects[0].attachments[0].id, "att-child");

    let section_tree = pages
        .iter()
        .find(|p| p.page_id.is_none())
        .expect("section tree exists");
    assert_eq!(section_tree.objects.len(), 1);
    assert_eq!(section_tree.objects[0].object_id, "obj-standalone");
    assert_eq!(section_tree.objects[0].attachments.len(), 1);
    assert_eq!(section_tree.objects[0].attachments[0].id, "att-act");

    // 回收站视图：只含已删除附件
    let trash_pages = build_attachment_tree_pages(
        &vault,
        &page_objects,
        &section_groups,
        &children_by_parent,
        true,
    )
    .unwrap();
    let trash_tree = trash_pages
        .iter()
        .find(|p| p.page_id.is_none())
        .expect("trash section tree exists");
    assert_eq!(trash_tree.objects[0].attachments.len(), 1);
    assert_eq!(trash_tree.objects[0].attachments[0].id, "att-del");
}

#[test]
fn test_make_unique_dest_path_no_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("a.pdf");
    assert_eq!(make_unique_dest_path(&dest), dest);
}

#[test]
fn test_make_unique_dest_path_with_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("a.pdf");
    std::fs::write(&dest, b"").unwrap();
    let r1 = make_unique_dest_path(&dest);
    assert_eq!(r1, tmp.path().join("a(1).pdf"));
    std::fs::write(&r1, b"").unwrap();
    let r2 = make_unique_dest_path(&dest);
    assert_eq!(r2, tmp.path().join("a(2).pdf"));
}

#[test]
fn test_make_unique_dest_path_fixes_system_suffix() {
    let tmp = tempfile::tempdir().unwrap();
    // 系统保存对话框可能自动返回 a.pdf(1)，需要修正为 a(1).pdf
    let dest = tmp.path().join("a.pdf(1)");
    let result = make_unique_dest_path(&dest);
    assert_eq!(result, tmp.path().join("a(1).pdf"));
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[test]
fn test_copy_into_dir_same_name_no_overwrite() {
    // 同名附件冲突：对象1与对象2各有名为 "2" 的附件（内容不同），分享时
    // 不得互相覆盖——第二次分享应生成序号副本 a(1)。
    let tmp = tempfile::tempdir().unwrap();
    let src1 = tmp.path().join("src-1");
    let src2 = tmp.path().join("src-2");
    std::fs::write(&src1, b"content-A").unwrap();
    std::fs::write(&src2, b"content-B").unwrap();
    let att_key = [0x42u8; 32];

    let r1 = copy_into_dir(tmp.path(), &src1, "2", &att_key).unwrap();
    let r2 = copy_into_dir(tmp.path(), &src2, "2", &att_key).unwrap();

    // 两次分享同名附件得到不同路径，内容互不覆盖
    assert_ne!(r1, r2);
    assert_eq!(std::fs::read(&r1).unwrap(), b"content-A");
    assert_eq!(std::fs::read(&r2).unwrap(), b"content-B");
    // 第二次生成序号副本
    assert_eq!(r1, tmp.path().join("2"));
    assert_eq!(r2, tmp.path().join("2(1)"));
}

#[test]
fn test_sanitize_duplicate_suffix_variants() {
    assert_eq!(sanitize_duplicate_suffix("a.pdf(1)"), "a(1).pdf");
    assert_eq!(sanitize_duplicate_suffix("a (1).pdf"), "a(1).pdf");
    assert_eq!(sanitize_duplicate_suffix("a(1)"), "a(1)");
    assert_eq!(sanitize_duplicate_suffix("a.pdf"), "a.pdf");
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[test]
fn test_cleanup_share_dir_removes_old_plaintext_copies() {
    // P010: 分享前清理旧副本——上次分享的明文副本不得无限累积残留。
    let tmp = tempfile::tempdir().unwrap();
    // 造旧残留：一个明文文件 + 一个子目录（子目录应保留，仅清理平铺文件）
    std::fs::write(tmp.path().join("old.png"), b"plaintext-1").unwrap();
    std::fs::write(tmp.path().join("old(1).pdf"), b"plaintext-2").unwrap();
    std::fs::create_dir(tmp.path().join("subdir")).unwrap();

    cleanup_share_dir(tmp.path());

    // 旧明文副本被删除
    assert!(!tmp.path().join("old.png").exists());
    assert!(!tmp.path().join("old(1).pdf").exists());
    // 子目录保留（目录本身由后续 copy_into_dir 复用）
    assert!(tmp.path().join("subdir").is_dir());
}

/// P001-3/P010: `decrypt_to_temp_dir`——解密到一次性 UUID 子目录、不同调用
/// 路径互不覆盖，且后台延迟清理（grace 过后文件与子目录被删除）。
#[test]
fn test_decrypt_to_temp_dir_unique_and_delayed_cleanup() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("secret.bin");
    let plain = b"temp-copy-plaintext".repeat(10);
    std::fs::write(&src, &plain).unwrap();
    let att_key = [0x42u8; 32];

    // 两次调用 → 不同子目录（互不覆盖，消除并发竞态）。
    let d1 = decrypt_to_temp_dir(
        &att_key,
        &src,
        "a.pdf",
        "solosoul_open",
        Duration::from_secs(30 * 60),
    )
    .unwrap();
    let d2 = decrypt_to_temp_dir(
        &att_key,
        &src,
        "a.pdf",
        "solosoul_open",
        Duration::from_secs(30 * 60),
    )
    .unwrap();
    assert_ne!(d1.parent(), d2.parent(), "每次调用应生成独立子目录");
    assert!(d1.exists() && d2.exists());
    assert!(d1.starts_with(std::env::temp_dir().join("solosoul_open")));
    // 内容解密一致
    assert_eq!(
        solosoul_core::attachment_crypto::read_file_decrypted(&att_key, &d1, 10_000).unwrap(),
        plain
    );

    // 短 grace（300ms）→ 延迟清理生效：文件与子目录被删除。
    let d3 = decrypt_to_temp_dir(
        &att_key,
        &src,
        "b.bin",
        "solosoul_open",
        Duration::from_millis(300),
    )
    .unwrap();
    let parent = d3.parent().unwrap().to_path_buf();
    assert!(d3.exists());
    std::thread::sleep(Duration::from_millis(900));
    assert!(!d3.exists(), "grace 过后临时明文应被删除");
    assert!(!parent.exists(), "grace 过后子目录应被删除");
}

// ── resolve_verified_attachment_path（attachment_open / attachment_share 共享路径） ──

/// 创建已解锁的 VaultService + 一个含真实附件文件的对象，返回 (svc, dir, 附件文件路径)。
fn setup_unlocked_attachment() -> (
    solosoul_core::vault_service::VaultService,
    TempDir,
    std::path::PathBuf,
) {
    let dir = TempDir::new().unwrap();
    let base = dir.path().join("vault");
    let svc = solosoul_core::vault_service::VaultService::with_base_path(base.clone());
    svc.create_account_with_id("acc-1", "acc-1", "pw1234567890", None)
        .unwrap();
    svc.unlock("acc-1", "pw1234567890").unwrap();

    // 在 vault attachments 目录下创建真实附件文件
    let att_dir = base.join("attachments").join("obj-1").join("att-1");
    std::fs::create_dir_all(&att_dir).unwrap();
    let file_path = att_dir.join("a.pdf");
    std::fs::write(&file_path, b"hello").unwrap();

    let att = AttachmentMeta {
        id: "att-1".to_string(),
        object_id: "obj-1".to_string(),
        file_name: "a.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        size_bytes: 5,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        deleted_at: None,
        src_path: None,
        vault_path: Some(file_path.to_string_lossy().to_string()),
        description: None,
        tags: vec![],
    };
    let record = ObjectRecord {
        contract_type_id: None,
        id: "obj-1".to_string(),
        account_id: "acc-1".to_string(),
        type_id: "note".to_string(),
        section_type: "identity".to_string(),
        name: "obj-1".to_string(),
        icon_name: "document".to_string(),
        parent_id: None,
        children_ids: vec![],
        properties: serde_json::json!({ "__attachments": [att] }),
        property_labels: None,
        sensitivity_level: "internal".to_string(),
        is_deleted: false,
        deleted_at: None,
        tags_json: vec![],
        template_id: None,
        template_type: None,
        template_hash: None,
        ignored_template_hash: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
        version: 1,
    };
    let vault = svc.get_vault_store().unwrap();
    vault.save_object(&record).unwrap();

    (svc, dir, file_path)
}

/// 改写对象附件的 vault_path（用于构造越界 / 含 `..` 的用例）。
fn set_attachment_path(svc: &solosoul_core::vault_service::VaultService, new_path: &str) {
    let vault = svc.get_vault_store().unwrap();
    let mut record = vault.load_object("obj-1").unwrap().unwrap();
    let mut atts = load_attachments(&record.properties);
    atts[0].vault_path = Some(new_path.to_string());
    save_attachments(&mut record.properties, &atts);
    vault.save_object(&record).unwrap();
}

#[test]
fn test_resolve_verified_attachment_path_resolves_inside_vault() {
    let (svc, _dir, file_path) = setup_unlocked_attachment();
    let (path, att) = resolve_verified_attachment_path(&svc, "obj-1", "att-1").unwrap();
    assert_eq!(path, file_path.canonicalize().unwrap());
    assert_eq!(att.id, "att-1");
    assert_eq!(att.file_name, "a.pdf");
}

#[test]
fn test_resolve_verified_attachment_path_rejects_outside_vault() {
    let (svc, dir, _file_path) = setup_unlocked_attachment();
    // 附件路径指向 vault 外部的真实文件（canonicalize 成功但不在 attachments 内）
    let outside = dir.path().join("outside.txt");
    std::fs::write(&outside, b"x").unwrap();
    set_attachment_path(&svc, &outside.to_string_lossy());

    let err = resolve_verified_attachment_path(&svc, "obj-1", "att-1").unwrap_err();
    assert!(err.contains("outside vault storage"), "{err}");
}

#[test]
fn test_resolve_verified_attachment_path_rejects_parent_dir() {
    let (svc, _dir, file_path) = setup_unlocked_attachment();
    // 构造含 `..` 的原始路径，但 canonicalize 后仍指向真实文件（存在才走到 `..` 分支）
    let att_dir = file_path.parent().unwrap();
    let raw = att_dir
        .join("..")
        .join("..")
        .join("obj-1")
        .join("att-1")
        .join("a.pdf");
    set_attachment_path(&svc, &raw.to_string_lossy());

    let err = resolve_verified_attachment_path(&svc, "obj-1", "att-1").unwrap_err();
    assert!(err.contains("must not contain '..'"), "{err}");
}

#[test]
fn test_resolve_verified_attachment_path_missing_entities() {
    let (svc, _dir, _file_path) = setup_unlocked_attachment();
    let err = resolve_verified_attachment_path(&svc, "obj-1", "att-missing").unwrap_err();
    assert!(err.contains("Attachment not found"), "{err}");
    let err = resolve_verified_attachment_path(&svc, "obj-missing", "att-1").unwrap_err();
    assert!(err.contains("Object not found"), "{err}");
}

#[test]
fn test_resolve_verified_attachment_path_missing_file() {
    let (svc, _dir, file_path) = setup_unlocked_attachment();
    // vault_path 指向不存在的文件 → canonicalize 失败且文件不存在
    set_attachment_path(
        &svc,
        &file_path.with_file_name("missing.pdf").to_string_lossy(),
    );
    let err = resolve_verified_attachment_path(&svc, "obj-1", "att-1").unwrap_err();
    assert!(err.contains("Cannot access attachment file"), "{err}");
}
