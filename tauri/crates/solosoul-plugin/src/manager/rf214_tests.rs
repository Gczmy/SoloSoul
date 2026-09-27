use super::*;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ID: &str = "com.solosoul.rf214.manager";
const VERSION: &str = "1.0.0";
const WASM: &[u8] = b"\0asm\x01\0\0\0";
const OTHER_WASM: &[u8] = b"\0asm\x01\0\0\0\0\x02\x01x";
const WAIT: Duration = Duration::from_secs(5);

struct Fixture {
    manager: PluginManager,
    directory: tempfile::TempDir,
    market: PathBuf,
    data: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let market = directory.path().join("market");
        let data = directory.path().join("data");
        std::fs::create_dir_all(&market).unwrap();
        let mut manager = PluginManager::new_with_dirs(market.clone(), data.clone()).unwrap();
        manager.install_client = Some(reqwest::Client::builder().no_proxy().build().unwrap());
        Self {
            manager,
            directory,
            market,
            data,
        }
    }

    fn registry(&self, url: Option<&str>, hash: &str) {
        let info = serde_json::json!({
            "sha256": hash, "min_app_version": "0.0.0", "max_app_version": "99.0.0",
            "download_url": url, "raw_url": url,
        });
        self.write_registry(serde_json::json!({ (VERSION): info }));
    }

    fn write_registry(&self, versions: serde_json::Value) {
        std::fs::write(self.market.join("registry.json"), serde_json::json!({
            "plugins": { (ID): { "name": "RF214 synthetic", "latest_version": VERSION, "versions": versions } }
        }).to_string()).unwrap();
    }

    fn bundled(&self, id: &str, version: &str, wasm: &[u8]) {
        let target = self.market.join("plugins").join(ID);
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("manifest.json"), raw_manifest(id, version)).unwrap();
        std::fs::write(target.join("plugin.wasm"), wasm).unwrap();
    }

    fn save_installed(&self, version: &str, wasm: &[u8]) {
        let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "id": ID, "name": "RF214 installed", "version": version, "description": "synthetic",
            "wasmHashSha256": compute_sha256(wasm)
        }))
        .unwrap();
        self.manager.store.save_plugin(&manifest, wasm).unwrap();
    }

    fn installed_audits(&self) -> usize {
        self.manager
            .audit
            .recent(100)
            .iter()
            .filter(|entry| matches!(entry.action, PluginAuditAction::PluginInstalled { .. }))
            .count()
    }
}

fn raw_manifest(id: &str, version: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "plugin_id": id, "name": "RF214 合成插件", "version": version,
        "description": "synthetic manifest", "required_fields": ["public.name"],
        "optional_fields": ["contact.email"], "require_user_confirmation": true,
        "network_policy": { "blockAllOutbound": true, "allowedDomains": [] }
    }))
    .unwrap()
}

fn response(body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

/// 每个请求和退出均有超时；断言失败时 Drop 撤销测试服务器，不留下接受连接的任务。
struct Source {
    url: String,
    worker: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

impl Source {
    async fn start(replies: Vec<(&'static str, Vec<u8>)>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/plugin.wasm", listener.local_addr().unwrap());
        let worker = tokio::spawn(async move {
            for (path, reply) in replies {
                let (mut stream, _) = tokio::time::timeout(WAIT, listener.accept())
                    .await
                    .map_err(|_| "RF214 server accept timed out".to_string())?
                    .map_err(|_| "RF214 server accept failed".to_string())?;
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0u8; 1024];
                    let count = tokio::time::timeout(WAIT, stream.read(&mut chunk))
                        .await
                        .map_err(|_| "RF214 server request timed out".to_string())?
                        .map_err(|_| "RF214 server request failed".to_string())?;
                    if count == 0 {
                        return Err("RF214 request closed before headers".to_string());
                    }
                    request.extend_from_slice(&chunk[..count]);
                    if request.windows(4).any(|value| value == b"\r\n\r\n") {
                        break;
                    }
                    if request.len() > 16 * 1024 {
                        return Err("RF214 request headers too long".to_string());
                    }
                }
                let request = String::from_utf8_lossy(&request);
                if request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    != Some(path)
                {
                    return Err("RF214 received unexpected request path".into());
                }
                // 输入上限测试会在响应尚未写完时关闭连接，这是期望的读取边界。
                let _ = tokio::time::timeout(WAIT, stream.write_all(&reply)).await;
                let _ = stream.shutdown().await;
            }
            Ok(())
        });
        Self {
            url,
            worker: Some(worker),
        }
    }

    async fn finish(&mut self) {
        tokio::time::timeout(WAIT, self.worker.as_mut().unwrap())
            .await
            .expect("RF214 source did not exit")
            .expect("RF214 source panicked")
            .expect("RF214 source request mismatch");
        self.worker.take();
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.abort();
        }
    }
}

fn files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn collect(base: &Path, path: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(base, &path, result);
            } else {
                result.push((
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut result = Vec::new();
    collect(root, root, &mut result);
    result.sort_by(|left, right| left.0.cmp(&right.0));
    result
}

#[tokio::test]
async fn rf214_remote_prepare_drop_preserves_installed_pair_and_has_no_success() {
    let fixture = Fixture::new();
    fixture.save_installed("0.9.0", OTHER_WASM);
    let mut source = Source::start(vec![
        ("/manifest.json", response(&raw_manifest(ID, VERSION))),
        ("/plugin.wasm", response(WASM)),
    ])
    .await;
    fixture.registry(Some(&source.url), &compute_sha256(WASM));
    let before = files(&fixture.data);
    let events = Mutex::new(Vec::new());
    let emit = |event| events.lock().unwrap().push(event);
    let prepared = fixture
        .manager
        .prepare_update_with_progress(ID, &emit)
        .await
        .unwrap();
    source.finish().await;
    assert_eq!(prepared.manifest().id, ID);
    assert_eq!(prepared.manifest().version, VERSION);
    assert_eq!(prepared.manifest().name, "RF214 合成插件");
    assert_eq!(
        prepared.manifest().permissions,
        ["public.name", "contact.email"]
    );
    assert!(prepared.manifest().require_user_confirmation);
    assert!(prepared.manifest().network_policy.block_all_outbound);
    assert!(prepared
        .manifest()
        .network_policy
        .allowed_domains
        .is_empty());
    let (old, bytes) = fixture.manager.store.load_plugin(ID).unwrap();
    assert_eq!(old.version, "0.9.0");
    assert_eq!(bytes, OTHER_WASM);
    assert_eq!(fixture.installed_audits(), 0);
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .all(|event: &PluginInstallProgress| event.percent < 100));
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event.phase == PluginInstallPhase::Finalizing));
    drop(prepared);
    assert_eq!(
        files(&fixture.data),
        before,
        "dropping preparation must release its files"
    );
    assert_eq!(
        fixture.manager.list_installed().unwrap()[0].version,
        "0.9.0"
    );
}

#[tokio::test]
async fn rf214_gui_wrapper_emits_completed_after_pair_and_audit_are_published() {
    let fixture = Fixture::new();
    let mut source = Source::start(vec![
        ("/manifest.json", response(&raw_manifest(ID, VERSION))),
        ("/plugin.wasm", response(WASM)),
    ])
    .await;
    fixture.registry(Some(&source.url), &compute_sha256(WASM));
    let events = Mutex::new(Vec::new());
    let emit = |event: PluginInstallProgress| {
        if event.phase == PluginInstallPhase::Completed {
            let (manifest, wasm) = fixture.manager.store.load_plugin(ID).unwrap();
            assert_eq!(manifest.version, VERSION);
            assert_eq!(wasm, WASM);
            assert_eq!(fixture.installed_audits(), 1);
        }
        events.lock().unwrap().push(event);
    };
    let result = fixture
        .manager
        .update_with_progress(ID, &emit)
        .await
        .unwrap();
    source.finish().await;
    assert_eq!(result.plugin_id, ID);
    assert_eq!(result.version, VERSION);
    let first = events.lock().unwrap().clone();
    assert_eq!(
        first
            .iter()
            .filter(|event| event.phase == PluginInstallPhase::Completed)
            .count(),
        1
    );
    assert_eq!(first.last().unwrap().downloaded_bytes, WASM.len() as u64);
    assert_eq!(first.last().unwrap().total_bytes, Some(WASM.len() as u64));
    assert!(first
        .windows(2)
        .all(|pair| pair[0].percent <= pair[1].percent));
    // 服务器已退出；真实同版本+SHA快速复用无需再次下载，也不重复安装审计。
    fixture
        .manager
        .install_from_registry_with_progress(ID, VERSION, &emit)
        .await
        .unwrap();
    assert_eq!(fixture.installed_audits(), 1);
    assert_eq!(
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.phase == PluginInstallPhase::Completed)
            .count(),
        2
    );
}

#[tokio::test]
async fn rf214_reuse_rechecks_removed_or_replaced_installation_at_publish() {
    for action in ["unchanged", "remove", "replace"] {
        let fixture = Fixture::new();
        fixture.registry(None, &compute_sha256(WASM));
        fixture.save_installed(VERSION, WASM);
        let events = Mutex::new(Vec::new());
        let prepared = fixture
            .manager
            .prepare_install_from_registry_with_progress(ID, VERSION, &|event| {
                events.lock().unwrap().push(event);
            })
            .await
            .unwrap();
        match action {
            "remove" => fixture.manager.store.delete_plugin(ID).unwrap(),
            "replace" => fixture.save_installed("2.0.0", OTHER_WASM),
            _ => {}
        }
        let result = fixture.manager.publish_install(prepared);
        match action {
            "unchanged" => assert_eq!(result.unwrap().version, VERSION),
            "remove" => {
                assert!(result.is_err());
                assert!(fixture.manager.list_installed().unwrap().is_empty());
            }
            "replace" => {
                assert!(result.is_err());
                let (manifest, wasm) = fixture.manager.store.load_plugin(ID).unwrap();
                assert_eq!(manifest.version, "2.0.0");
                assert_eq!(wasm, OTHER_WASM);
            }
            _ => unreachable!(),
        }
        assert_eq!(fixture.installed_audits(), 0);
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .all(|event| event.phase != PluginInstallPhase::Completed));
    }
}

#[tokio::test]
async fn rf214_remote_manifest_identity_mismatch_cannot_fallback_or_publish() {
    for (id, version) in [("com.solosoul.other", VERSION), (ID, "2.0.0")] {
        let fixture = Fixture::new();
        let mut source = Source::start(vec![(
            "/manifest.json",
            response(&raw_manifest(id, version)),
        )])
        .await;
        fixture.registry(Some(&source.url), &compute_sha256(WASM));
        fixture.bundled(ID, VERSION, WASM);
        let result = fixture
            .manager
            .prepare_update_with_progress(ID, &|_| {})
            .await;
        source.finish().await;
        assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
        assert!(fixture.manager.list_installed().unwrap().is_empty());
        assert_eq!(fixture.installed_audits(), 0);
    }
}

#[tokio::test]
async fn rf214_bad_checksum_and_oversized_wasm_never_emit_completed_or_install() {
    for oversized in [false, true] {
        let fixture = Fixture::new();
        let wasm_reply = if oversized {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                MAX_WASM_SIZE + 1
            )
            .into_bytes()
        } else {
            response(OTHER_WASM)
        };
        let mut source = Source::start(vec![
            ("/manifest.json", response(&raw_manifest(ID, VERSION))),
            ("/plugin.wasm", wasm_reply),
        ])
        .await;
        fixture.registry(Some(&source.url), &compute_sha256(WASM));
        fixture.bundled(ID, VERSION, WASM);
        let events = Mutex::new(Vec::new());
        let result = fixture
            .manager
            .install_from_registry_with_progress(ID, VERSION, &|event| {
                events.lock().unwrap().push(event);
            })
            .await;
        source.finish().await;
        if oversized {
            assert!(matches!(result, Err(PluginError::WasmTooLarge(_))));
        } else {
            assert!(matches!(result, Err(PluginError::ChecksumMismatch)));
        }
        assert!(fixture.manager.list_installed().unwrap().is_empty());
        assert_eq!(fixture.installed_audits(), 0);
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .all(|event| event.percent < 100));
    }
}

#[tokio::test]
async fn rf214_bundled_fallback_reports_actual_version_and_enforces_identity_and_compatibility() {
    for mode in ["valid", "wrong-id", "incompatible"] {
        let fixture = Fixture::new();
        let hash = compute_sha256(WASM);
        fixture.write_registry(serde_json::json!({
            (VERSION): { "sha256": hash, "min_app_version": "0.0.0", "max_app_version": "99.0.0" },
            "0.9.0": { "sha256": hash, "min_app_version": if mode == "incompatible" { "99.0.0" } else { "0.0.0" }, "max_app_version": "100.0.0" }
        }));
        fixture.bundled(
            if mode == "wrong-id" {
                "com.solosoul.other"
            } else {
                ID
            },
            "0.9.0",
            WASM,
        );
        let events = Mutex::new(Vec::new());
        let prepared = fixture
            .manager
            .prepare_update_with_progress(ID, &|event| events.lock().unwrap().push(event))
            .await;
        assert!(fixture.manager.list_installed().unwrap().is_empty());
        assert_eq!(fixture.installed_audits(), 0);
        assert!(events
            .lock()
            .unwrap()
            .iter()
            .all(|event| event.percent < 100));
        match mode {
            "valid" => {
                let prepared = prepared.unwrap();
                assert_eq!(prepared.manifest().version, "0.9.0");
                let result = fixture.manager.publish_install(prepared).unwrap();
                assert_eq!(result.version, "0.9.0");
                assert_eq!(fixture.manager.store.load_plugin(ID).unwrap().1, WASM);
                assert_eq!(fixture.installed_audits(), 1);
            }
            "wrong-id" => assert!(matches!(prepared, Err(PluginError::InvalidManifest(_)))),
            "incompatible" => assert!(matches!(prepared, Err(PluginError::IncompatibleVersion(_)))),
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn rf214_unknown_length_inputs_are_bounded_before_acceptance() {
    for wasm in [false, true] {
        let limit = if wasm {
            MAX_WASM_SIZE
        } else {
            MAX_MANIFEST_SIZE
        };
        let mut reply = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        reply.extend(std::iter::repeat_n(b'x', limit + 1));
        let mut source = Source::start(vec![("/plugin.wasm", reply)]).await;
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(&source.url)
            .send()
            .await
            .unwrap();
        assert_eq!(response.content_length(), None);
        let result = if wasm {
            read_wasm_response(response, &mut InstallProgressReporter::new(&|_| {})).await
        } else {
            read_manifest_response(response).await
        };
        source.finish().await;
        if wasm {
            assert!(matches!(result, Err(PluginError::WasmTooLarge(_))));
        } else {
            assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
        }
    }
}

#[tokio::test]
async fn rf214_prepared_install_cannot_be_published_into_another_store() {
    let first = Fixture::new();
    let second = Fixture::new();
    first.registry(None, &compute_sha256(WASM));
    first.bundled(ID, VERSION, WASM);
    let before = files(&first.data);
    let prepared = first
        .manager
        .prepare_update_with_progress(ID, &|_| {})
        .await
        .unwrap();
    assert!(second.manager.publish_install(prepared).is_err());
    assert!(first.manager.list_installed().unwrap().is_empty());
    assert!(second.manager.list_installed().unwrap().is_empty());
    assert_eq!(first.installed_audits(), 0);
    assert_eq!(second.installed_audits(), 0);
    assert_eq!(files(&first.data), before);
    assert!(first.directory.path().exists());
    assert!(second.directory.path().exists());
}
