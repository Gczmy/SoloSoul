//! RF905：插件实际目录 owner、准备结果清理及真实 Wasm blocking worker 生命周期。
//! 仅 TempDir/合成 Vault，不读取用户目录或远程市场；草稿尚未执行。
use super::*;
use solosoul_core::import_activity::begin_owned_root_maintenance;
use solosoul_vault::VaultConfig;
use std::sync::{Mutex, Weak};
use std::time::Duration;

const ID: &str = "com.solosoul.rf905.worker";
const WAIT: Duration = Duration::from_secs(15);
const BARRIER: &str = "RF905 actual worker barrier";
const MINIMAL_WASM: &[u8] = b"\0asm\x01\0\0\0";

fn manifest(wasm: &[u8]) -> PluginManifest {
    serde_json::from_value(serde_json::json!({
        "id": ID, "name": "RF905 synthetic", "version": "1.0.0", "description": "synthetic",
        "wasmHashSha256": compute_sha256(wasm)
    }))
    .unwrap()
}

fn owned_manager(market: &Path, owner: Arc<VaultRootOwner>) -> PluginManager {
    PluginManager::new_with_dirs_owned(market.to_path_buf(), owner.root().to_path_buf(), owner)
        .unwrap()
}

#[test]
fn owned_plugin_store_rejects_foreign_root_before_business_files() {
    let directory = tempfile::tempdir().unwrap();
    let owner = VaultRootOwner::acquire(&directory.path().join("owned")).unwrap();
    let other = directory.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    let error = PluginStore::new_with_data_dir_owned(other.clone(), owner)
        .err()
        .unwrap();
    assert!(error.to_string().contains("VAULT_ROOT_MISMATCH"));
    assert!(!other.join("plugins").exists());
}

#[test]
fn prepared_stage_pins_actual_root_and_activity_after_manager_drop() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("plugins-root");
    let owner = VaultRootOwner::acquire(&root).unwrap();
    let weak = Arc::downgrade(&owner);
    let manager = owned_manager(&directory.path().join("market"), owner.clone());
    // 真正 Store 准备结果拥有版本 TempDir 和 current TempPath；不替换被测存储。
    let prepared = manager
        .store
        .prepare_plugin(&manifest(MINIMAL_WASM), MINIMAL_WASM)
        .unwrap();
    let plugin = root.join("plugins").join(ID);
    let versions = plugin.join("versions");
    let stage = std::fs::read_dir(&versions)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(stage.join("plugin.wasm").exists());
    drop(manager);
    drop(owner);
    let pinned = weak
        .upgrade()
        .expect("prepared stage must pin actual owner");
    assert_eq!(
        begin_owned_root_maintenance(pinned.clone()).err().unwrap(),
        "IMPORT_OPERATIONS_ACTIVE"
    );
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    assert_eq!(
        VaultRootOwner::acquire(&root).err().unwrap(),
        "VAULT_DIRECTORY_BUSY"
    );
    drop(pinned);
    drop(prepared);
    assert!(
        !stage.exists(),
        "stage cleanup must finish before root release"
    );
    assert!(!plugin.join("current.json").exists());
    assert_eq!(weak.strong_count(), 0);
    let fresh = VaultRootOwner::acquire(&root).unwrap();
    assert!(begin_owned_root_maintenance(fresh).is_ok());
}

#[tokio::test]
async fn owned_manager_maintenance_rejects_writes_before_audit_or_stage() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("global");
    let owner = VaultRootOwner::acquire(&root).unwrap();
    let manager = owned_manager(&directory.path().join("market"), owner.clone());
    manager
        .store
        .save_plugin(&manifest(MINIMAL_WASM), MINIMAL_WASM)
        .unwrap();
    let before = std::fs::read(root.join("plugins").join(ID).join("current.json")).unwrap();
    let maintenance = begin_owned_root_maintenance(owner).unwrap();
    let error = manager.uninstall(ID).unwrap_err();
    assert!(error.to_string().contains("IMPORT_DIRECTORY_BUSY"));
    assert_eq!(
        std::fs::read(root.join("plugins").join(ID).join("current.json")).unwrap(),
        before
    );
    assert!(!root.join("plugin_audit.jsonl").exists());
    let error = manager
        .prepare_install_from_registry_with_progress(ID, "1.0.0", &|_| {})
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("IMPORT_DIRECTORY_BUSY"));
    let error = manager
        .run(ID, HashMap::new(), Arc::new(NoopSink), None, None, None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("IMPORT_DIRECTORY_BUSY"));
    assert!(!root.join("plugin_audit.jsonl").exists());
    drop(maintenance);
    manager.uninstall(ID).unwrap();
    assert!(manager.list_installed().unwrap().is_empty());
}

struct NoopSink;
impl PluginEventSink for NoopSink {
    fn send(&self, _: PluginEvent) -> Result<(), String> {
        Ok(())
    }
}

// 无新 WAT 依赖，手工组装真实 Wasm：run() -> i32，调用 env.solosoul_log 后返回。
fn barrier_wasm() -> Vec<u8> {
    fn leb(mut value: usize, output: &mut Vec<u8>) {
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            output.push(if value == 0 { byte } else { byte | 0x80 });
            if value == 0 {
                break;
            }
        }
    }
    fn name(value: &str, output: &mut Vec<u8>) {
        leb(value.len(), output);
        output.extend_from_slice(value.as_bytes());
    }
    fn section(kind: u8, payload: &[u8], output: &mut Vec<u8>) {
        output.push(kind);
        leb(payload.len(), output);
        output.extend_from_slice(payload);
    }
    assert!(BARRIER.len() < 64);
    let mut wasm = MINIMAL_WASM.to_vec();
    section(
        1,
        &[2, 0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 0, 0x60, 0, 1, 0x7f],
        &mut wasm,
    );
    let mut import = vec![1];
    name("env", &mut import);
    name("solosoul_log", &mut import);
    import.extend_from_slice(&[0, 0]);
    section(2, &import, &mut wasm);
    section(3, &[1, 1], &mut wasm);
    section(5, &[1, 0, 1], &mut wasm);
    let mut exports = vec![2];
    name("memory", &mut exports);
    exports.extend_from_slice(&[2, 0]);
    name("run", &mut exports);
    exports.extend_from_slice(&[0, 1]);
    section(7, &exports, &mut wasm);
    let body = vec![
        0,
        0x41,
        0,
        0x41,
        4,
        0x41,
        8,
        0x41,
        BARRIER.len() as u8,
        0x10,
        0,
        0x41,
        0,
        0x0b,
    ];
    let mut code = vec![1];
    leb(body.len(), &mut code);
    code.extend_from_slice(&body);
    section(10, &code, &mut wasm);
    let mut bytes = b"info\0\0\0\0".to_vec();
    bytes.extend_from_slice(BARRIER.as_bytes());
    let mut data = vec![1, 0, 0x41, 0, 0x0b];
    leb(bytes.len(), &mut data);
    data.extend_from_slice(&bytes);
    section(11, &data, &mut wasm);
    wasm
}

struct BarrierSink {
    entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl PluginEventSink for BarrierSink {
    fn send(&self, event: PluginEvent) -> Result<(), String> {
        let value: serde_json::Value =
            serde_json::from_str(&event.json_data).map_err(|e| e.to_string())?;
        if value.get("message").and_then(|v| v.as_str()) == Some(BARRIER) {
            if let Some(sender) = self.entered.lock().unwrap().take() {
                sender
                    .send(())
                    .map_err(|_| "RF905 barrier receiver gone".to_string())?;
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(WAIT)
                    .map_err(|_| "RF905 actual worker release timeout".to_string())?;
            }
        }
        Ok(())
    }
}

struct ReleaseOnDrop(Option<std::sync::mpsc::Sender<()>>);
impl ReleaseOnDrop {
    fn release(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

async fn wait_owner_release(owner: &Weak<VaultRootOwner>) {
    tokio::time::timeout(WAIT, async {
        while owner.strong_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("actual plugin worker must release root after return and Drop");
}

#[tokio::test]
async fn cancelling_real_run_keeps_native_and_global_roots_until_actual_wasm_returns() {
    let directory = tempfile::tempdir().unwrap();
    let native = directory.path().join("native");
    let global = directory.path().join("global");
    let native_owner = VaultRootOwner::acquire(&native).unwrap();
    let global_owner = VaultRootOwner::acquire(&global).unwrap();
    let native_weak = Arc::downgrade(&native_owner);
    let global_weak = Arc::downgrade(&global_owner);
    let account_path = native.join("synthetic-account");
    std::fs::create_dir_all(&account_path).unwrap();
    let vault = Arc::new(
        VaultStore::open_owned(
            VaultConfig::new("synthetic-account", account_path).with_data_key([7; 32]),
            native_owner.clone(),
        )
        .unwrap(),
    );
    let manager = Arc::new(owned_manager(
        &directory.path().join("market"),
        global_owner.clone(),
    ));
    let wasm = barrier_wasm();
    manager.store.save_plugin(&manifest(&wasm), &wasm).unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release_tx));
    let sink = Arc::new(BarrierSink {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
    });
    let manager_for_task = manager.clone();
    let vault_for_task = vault.clone();
    let task = tokio::spawn(async move {
        manager_for_task
            .run(
                ID,
                HashMap::new(),
                sink,
                Some(vault_for_task),
                Some("synthetic-account".into()),
                None,
            )
            .await
    });
    tokio::time::timeout(WAIT, entered_rx)
        .await
        .unwrap()
        .unwrap();
    let audit = global.join("plugin_audit.jsonl");
    let before = std::fs::read(&audit).unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(manager);
    drop(vault);
    drop(native_owner);
    drop(global_owner);
    for weak in [&native_weak, &global_weak] {
        let owner = weak
            .upgrade()
            .expect("real blocking worker must pin both roots");
        assert_eq!(
            begin_owned_root_maintenance(owner.clone()).err().unwrap(),
            "IMPORT_OPERATIONS_ACTIVE"
        );
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        assert_eq!(
            VaultRootOwner::acquire(owner.root()).err().unwrap(),
            "VAULT_DIRECTORY_BUSY"
        );
    }
    release.release();
    wait_owner_release(&native_weak).await;
    wait_owner_release(&global_weak).await;
    // log_impl 的审计发生在 barrier 返回后，证明取消 waiter 后仍有真实写入。
    let after = std::fs::read(&audit).unwrap();
    assert!(after.len() > before.len());
    assert!(VaultRootOwner::acquire(&native).is_ok());
    assert!(VaultRootOwner::acquire(&global).is_ok());
}

#[tokio::test]
async fn native_maintenance_is_not_hidden_by_different_owned_plugin_root() {
    let directory = tempfile::tempdir().unwrap();
    let native = directory.path().join("native");
    let global = directory.path().join("global");
    let native_owner = VaultRootOwner::acquire(&native).unwrap();
    let global_owner = VaultRootOwner::acquire(&global).unwrap();
    let account = native.join("synthetic-account");
    std::fs::create_dir_all(&account).unwrap();
    let vault = Arc::new(
        VaultStore::open_owned(
            VaultConfig::new("synthetic-account", account).with_data_key([7; 32]),
            native_owner.clone(),
        )
        .unwrap(),
    );
    let manager = owned_manager(&directory.path().join("market"), global_owner);
    manager
        .store
        .save_plugin(&manifest(MINIMAL_WASM), MINIMAL_WASM)
        .unwrap();
    let maintenance = begin_owned_root_maintenance(native_owner).unwrap();
    let error = manager
        .run(
            ID,
            HashMap::new(),
            Arc::new(NoopSink),
            Some(vault),
            Some("synthetic-account".into()),
            None,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("IMPORT_DIRECTORY_BUSY"));
    assert!(!global.join("plugin_audit.jsonl").exists());
    drop(maintenance);
}
