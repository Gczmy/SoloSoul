//! P025 Phase 3：两阶段改造前后 hold 基线对比工具（大数据集）。
//!
//! 用法：
//!   cargo test -p solosoul-vault --test p025_baseline -- --ignored --nocapture
//!
//! 本基准仅依赖 `VaultStore` 公开 API，可在改造前基线（如 60bf171f）与改造后
//! 同一运行，对比 `LockHoldObserver` 输出的 wait/hold：改造后 5 个 N 行解密热点
//! 的 hold 应仅覆盖 SQL 取数（不含锁外解密），明显低于改造前。
//!
//! 运行方式：在两个版本各自 `cargo test -p solosoul-vault --test p025_baseline
//! -- --ignored --nocapture`，对比输出中的 `hold=` 数值。
//!
//! 2026-08-20 实测对比（2000 对象 × ~16KB 明文 + 500 会话 + 快照）：
//!   热点                     改造前(60bf171f)   改造后(HEAD)   降幅
//!   list_objects             469ms             129ms         -72%
//!   list_object_records      427ms             106ms         -75%
//!   load_objects_batch       17ms              1ms           -94%
//!   list_conversations       10ms              1ms           -90%
//!   list_snapshots_with_data 1ms               0ms           ~-100%
//! 剩余 hold 为 SQL 取数 + 装箱大 payload 的固有成本；解密 + JSON 解析（约
//! 300~340ms）已完全移出锁区间，GUI 其他 DB 操作在解密期间不再被阻塞。

use solosoul_vault::{ObjectRecord, VaultConfig, VaultStore};
use std::io::Write;
use std::sync::{Arc, Mutex};

fn test_key() -> [u8; 32] {
    [0x42u8; 32]
}

/// 捕获型 fmt subscriber：把 debug 记录写进 Vec。
fn install_capture(logs: Arc<Mutex<Vec<String>>>) -> tracing::subscriber::DefaultGuard {
    struct VecWriterGuard(Arc<Mutex<Vec<String>>>);
    impl Write for VecWriterGuard {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(buf).to_string());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct VecWriter(Arc<Mutex<Vec<String>>>);
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for VecWriter {
        type Writer = VecWriterGuard;
        fn make_writer(&'a self) -> Self::Writer {
            VecWriterGuard(self.0.clone())
        }
    }
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(VecWriter(logs))
        .finish();
    tracing::subscriber::set_default(subscriber)
}

#[test]
#[ignore = "P025 Phase 3 数据收集工具：--ignored 手动运行（约 2s，大数据集）"]
fn p025_hold_baseline_large_dataset() {
    // 大数据集：2000 对象 × ~16KB 明文 properties + 500 会话 + 每对象 3 快照。
    // 数据量需让「解密+JSON 解析」在改造前 hold 中占显著比例，才可观测差异。
    let dir = tempfile::TempDir::new().unwrap();
    let config =
        VaultConfig::new("bench_account", dir.path().to_path_buf()).with_data_key(test_key());
    let vault = VaultStore::open(config).unwrap();

    let now = chrono::Utc::now().to_rfc3339();
    let big_props = serde_json::json!({
        "title": "benchmark-object",
        "content": "x".repeat(16 * 1024),
        "tags": (0..20).map(|i| format!("tag-{i}")).collect::<Vec<_>>(),
    });
    let ids: Vec<String> = (0..2000).map(|i| format!("bench-{i:04}")).collect();
    for id in &ids {
        let obj = ObjectRecord {
            id: id.clone(),
            account_id: "bench_account".to_string(),
            type_id: "note".to_string(),
            section_type: "identity".to_string(),
            name: format!("Benchmark Object {}", id),
            icon_name: "document".to_string(),
            parent_id: None,
            children_ids: vec![],
            properties: big_props.clone(),
            property_labels: None,
            sensitivity_level: "internal".to_string(),
            is_deleted: false,
            deleted_at: None,
            tags_json: vec![],
            template_id: None,
            template_type: None,
            template_hash: None,
            ignored_template_hash: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
            ..Default::default()
        };
        vault.save_object(&obj).unwrap();
    }
    for i in 0..500 {
        vault
            .save_conversation(
                "bench_account",
                &format!("conv-{i:04}"),
                &now,
                format!("{{\"messages\":[{}]}}", "x".repeat(2048)).as_bytes(),
            )
            .unwrap();
    }
    for id in ids.iter().take(200) {
        for s in 0..3 {
            vault
                .save_snapshot(id, "manual", b"{\"snapshot\":true}", "benchmark")
                .unwrap();
        }
    }

    // 安装捕获型 subscriber 后触发 5 个热点。
    let logs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let _guard = install_capture(logs.clone());

    let _ = vault.list_objects("bench_account", None, None, None, false, false);
    let _ = vault.list_object_records("bench_account");
    let _ = vault.load_objects_batch(&ids[..100]);
    let _ = vault.list_conversations("bench_account");
    let _ = vault.list_snapshots_with_data_batch(&ids[..100]);

    let joined = logs.lock().unwrap().join("\n");
    eprintln!("[P025 Phase 3] lock_observe 基线（大数据集）:\n{joined}");
    for label in [
        "fn=list_objects",
        "fn=list_object_records",
        "fn=load_objects_batch",
        "fn=list_conversations",
        "fn=list_snapshots_with_data_batch",
    ] {
        assert!(
            joined.contains(label),
            "LockHoldObserver 未对 {label} 发出观测日志, got:\n{joined}"
        );
    }
}
