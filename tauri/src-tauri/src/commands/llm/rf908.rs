//! RF908：真实 Tauri 命令宏/serde 参数绑定，临时 Vault + 无 Provider 的关键词回退。
//! Wry App 仅提供真实 AppHandle 路径 API，没有创建可见原生窗口；IPC 经 MockRuntime。
use crate::state::{recovery::RecoveryState, AppState};
use crate::sync::{
    auto_sync::AutoSyncManager, cloud_auto_sync::CloudAutoSyncManager,
    device_auto_sync::DeviceAutoSyncManager,
};
use serde_json::Value;
use solosoul_core::VaultService;
use solosoul_sync::SyncService;
use std::sync::{atomic::AtomicBool, Arc, Mutex, RwLock};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    webview::InvokeRequest,
    Manager,
};

fn invoke(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    body: Value,
) -> Result<Vec<super::GuideChunk>, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: "llm_search_guide_chunks".into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        },
    )
    .map(|response| response.deserialize().unwrap())
}

#[test]
fn rf908_actual_guide_command_binds_account_and_preserves_keyword_empty_and_locked_outcomes() {
    let _serial = crate::VAULT_TEST_LOCK.lock().unwrap();
    // 同步 IPC harness 需要后台 Tokio 调度器；显式 runtime 避免把全局测试互斥锁跨 await 持有。
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let _runtime_context = runtime.enter();
    let fixture: Value = serde_json::from_str(include_str!("rf908-requests.json")).unwrap();
    let account_id = fixture["bound"]["accountId"].as_str().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let vault_service = Arc::new(RwLock::new(VaultService::with_base_path(
        dir.path().join("vault"),
    )));
    {
        let service = vault_service.read().unwrap();
        service
            .create_account_with_id(account_id, "RF908 Synthetic", "Rf908SyntheticOnly1", None)
            .unwrap();
        service.unlock(account_id, "Rf908SyntheticOnly1").unwrap();
    }
    // 实际 AppHandle 的路径只使用独立标识；不读取/写入正常 SoloSoul 应用目录。
    let mut context = mock_context(noop_assets());
    context.config_mut().identifier =
        format!("com.solosoul.rf908.{}", uuid::Uuid::new_v4().simple());
    let native_app: tauri::App = tauri::Builder::default()
        .any_thread()
        .build(context)
        .unwrap();
    let handle = native_app.handle().clone();
    let sync_service = Arc::new(SyncService::new(vault_service.clone()));
    let owner = vault_service.read().unwrap().root_owner();
    let plugin_manager = Arc::new(
        crate::plugin::new_owned_plugin_manager_with_dirs(
            dir.path().join("market"),
            dir.path().join("plugins"),
            owner,
        )
        .unwrap(),
    );
    let state = AppState {
        handle: handle.clone(),
        vault_service: vault_service.clone(),
        ocr_jobs: Arc::new(crate::services::ocr_jobs::OcrJobs::new()),
        sync_service: sync_service.clone(),
        plugin_manager,
        auto_sync: AutoSyncManager::new_for_vault(vault_service.clone(), handle.clone()),
        device_auto_sync: DeviceAutoSyncManager::new(
            sync_service,
            vault_service.clone(),
            handle.clone(),
        ),
        cloud_auto_sync: CloudAutoSyncManager::new(vault_service.clone(), handle),
        trash_cleanup_running: Arc::new(AtomicBool::new(false)),
        recovery_state: Arc::new(Mutex::new(RecoveryState::new())),
        biometric_lockout_until: Arc::new(Mutex::new(None)),
    };
    let app = mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            super::rag::llm_search_guide_chunks
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "rf908", Default::default())
        .build()
        .unwrap();
    let legacy = invoke(&webview, fixture["legacy"].clone()).unwrap_err();
    assert!(legacy.as_str().unwrap().contains("accountId"), "{legacy}");
    let bound = invoke(&webview, fixture["bound"].clone()).unwrap();
    assert!(!bound.is_empty());
    assert!(bound.len() <= 3);
    assert!(bound
        .iter()
        .all(|chunk| !chunk.guide_id.is_empty() && !chunk.chunk_text.is_empty()));
    assert!(invoke(&webview, fixture["empty"].clone())
        .unwrap()
        .is_empty());
    vault_service.read().unwrap().lock();
    let locked = invoke(&webview, fixture["bound"].clone()).unwrap_err();
    assert_eq!(locked["code"], "VAULT_LOCKED");
    assert_eq!(locked["safeDetails"], Value::Null);
    assert_eq!(locked["retryable"], true);
    println!("RF908 actual binding: missingAccountRejected=true; boundGuideCount={}; topKZeroEmpty=true; lockedRejected=true", bound.len());
    let service_weak = Arc::downgrade(&vault_service);
    // 所有 IPC 已结算，测试没有持有 State 借用；Mock window 保留 manager，须显式取回装配的状态。
    // 仅限该受控测试收尾，不在生产运行期移除仍可能被命令借用的状态。
    #[allow(deprecated)]
    let owned_state = app.unmanage::<AppState>().unwrap();
    drop(owned_state);
    drop(webview);
    drop(app);
    drop(native_app);
    drop(vault_service);
    // 等待测试装配的空调度器收到 channel close，释放临时 Vault 后才清理目录。
    for _ in 0..100 {
        if service_weak.upgrade().is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(service_weak.upgrade().is_none());
    dir.close().unwrap();
}
