// P030: 跨设备恢复主机的运行时状态从 app_state.rs 拆分（对齐报告指引 state/recovery.rs）。
// 引用方仅 AppState.recovery_state 字段（Arc<Mutex<RecoveryState>>），外部经字段访问
// （类型推断，不直接命名类型），拆分不改变任何引用路径。
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct RecoveryState {
    /// 正向恢复（主机发送数据）的取消信号。
    pub host_cancel: Arc<AtomicBool>,
    pub host_thread: Option<std::thread::JoinHandle<()>>,
    pub export_path: Option<PathBuf>,
    /// 恢复主机注册的 mDNS 服务实例名（用于清理）。
    pub mdns_instance_name: Option<String>,
}

impl RecoveryState {
    pub fn new() -> Self {
        Self {
            host_cancel: Arc::new(AtomicBool::new(false)),
            host_thread: None,
            export_path: None,
            mdns_instance_name: None,
        }
    }
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::new()
    }
}
