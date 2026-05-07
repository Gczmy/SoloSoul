//! ONNX 模型加载与 Session 管理
//!
//! Phase 2: 支持 det + cls + rec 三模型并行加载。
//! 使用 `OnceCell<Mutex<ort::Session>>` 实现线程安全的全局单例。

use once_cell::sync::OnceCell;
use std::sync::Mutex;
use std::time::Instant;

use super::error::OcrError;

// ============================================================================
// 全局 ONNX Session 实例
// ============================================================================

static DET_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();
static CLS_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();
static REC_SESSION: OnceCell<Mutex<ort::session::Session>> = OnceCell::new();

/// Session 初始化时间戳（用于调试和监控）
static INIT_TIME: OnceCell<Instant> = OnceCell::new();

// ============================================================================
// 引擎状态
// ============================================================================

/// OCR 引擎状态（暴露给 Dart 端）
#[derive(Debug, Clone)]
pub struct OcrEngineStatus {
    pub is_loaded: bool,
    pub det_loaded: bool,
    pub cls_loaded: bool,
    pub rec_loaded: bool,
    pub uptime_secs: u64,
}

// ============================================================================
// 模型加载
// ============================================================================

/// Phase 2: 从内存字节加载三个 ONNX 模型（零文件复制）
///
/// `det_bytes`, `cls_bytes`, `rec_bytes` 分别为 det/cls/rec 模型的原始字节。
/// 任意一个传入空切片表示跳过该模型的加载（用于兼容 Phase 1 的仅 rec 模式）。
pub fn load_models_from_memory_v2(
    det_bytes: &[u8],
    cls_bytes: &[u8],
    rec_bytes: &[u8],
) -> Result<(), OcrError> {
    // 加载 det 模型（通用 OCR 需要，MRZ 不需要但 harmless）
    if !det_bytes.is_empty() && DET_SESSION.get().is_none() {
        match ort::session::Session::builder()
            .and_then(|mut b| b.commit_from_memory(det_bytes))
        {
            Ok(session) => {
                let _ = DET_SESSION.set(Mutex::new(session));
            }
            Err(e) => {
                eprintln!("[OCR] DET model load failed (non-fatal): {e}");
            }
        }
    }

    // 加载 cls 模型（方向分类，MRZ 不需要）
    // 该模型在当前 ort 版本下可能有兼容性问题，失败不阻塞
    if !cls_bytes.is_empty() && CLS_SESSION.get().is_none() {
        match ort::session::Session::builder()
            .and_then(|mut b| b.commit_from_memory(cls_bytes))
        {
            Ok(session) => {
                let _ = CLS_SESSION.set(Mutex::new(session));
            }
            Err(e) => {
                eprintln!("[OCR] CLS model load failed (non-fatal): {e}");
            }
        }
    }

    // 加载 rec 模型（文字识别，MRZ 必需）
    if !rec_bytes.is_empty() && REC_SESSION.get().is_none() {
        let session = ort::session::Session::builder()
            .map_err(|e| OcrError::InferenceFailed(format!("REC session builder failed: {e}")))?
            .commit_from_memory(rec_bytes)
            .map_err(|e| OcrError::InferenceFailed(format!("REC model load failed: {e}")))?;
        REC_SESSION
            .set(Mutex::new(session))
            .map_err(|_| OcrError::InferenceFailed("REC session already initialized".to_string()))?;
    }

    // 仅当至少一个模型成功加载时设置初始化时间
    if INIT_TIME.get().is_none()
        && (DET_SESSION.get().is_some() || CLS_SESSION.get().is_some() || REC_SESSION.get().is_some())
    {
        let _ = INIT_TIME.set(Instant::now());
    }

    Ok(())
}

/// Phase 1 兼容层：仅加载 rec 模型
pub fn load_models_from_memory(model_bytes: &[u8]) -> Result<(), OcrError> {
    load_models_from_memory_v2(&[], &[], model_bytes)
}

// ============================================================================
// Session 获取
// ============================================================================

pub fn get_det_session() -> Result<std::sync::MutexGuard<'static, ort::session::Session>, OcrError> {
    DET_SESSION
        .get()
        .ok_or(OcrError::ModelNotLoaded)?
        .lock()
        .map_err(|_| OcrError::InferenceFailed("DET session lock poisoned".to_string()))
}

pub fn get_cls_session() -> Result<std::sync::MutexGuard<'static, ort::session::Session>, OcrError> {
    CLS_SESSION
        .get()
        .ok_or(OcrError::ModelNotLoaded)?
        .lock()
        .map_err(|_| OcrError::InferenceFailed("CLS session lock poisoned".to_string()))
}

pub fn get_rec_session() -> Result<std::sync::MutexGuard<'static, ort::session::Session>, OcrError> {
    REC_SESSION
        .get()
        .ok_or(OcrError::ModelNotLoaded)?
        .lock()
        .map_err(|_| OcrError::InferenceFailed("REC session lock poisoned".to_string()))
}

// ============================================================================
// 资源管理与状态查询
// ============================================================================

/// 释放 OCR 引擎资源
///
/// **注意**：`OnceCell` 不支持安全重置。生产环境建议：
/// 1. 进入后台时不释放，仅暂停接受新的推理请求
/// 2. 内存紧张时由 OS 决定进程回收
/// 3. 如需显式释放，改用 `parking_lot::RwLock<Option<Arc<Session>>>`
pub fn unload_models() {
    // 当前实现为空，Session 随进程生命周期保持
}

/// 查询引擎状态（用于调试和 Dart 端状态展示）
pub fn engine_status() -> OcrEngineStatus {
    OcrEngineStatus {
        is_loaded: REC_SESSION.get().is_some(),
        det_loaded: DET_SESSION.get().is_some(),
        cls_loaded: CLS_SESSION.get().is_some(),
        rec_loaded: REC_SESSION.get().is_some(),
        uptime_secs: INIT_TIME
            .get()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0),
    }
}
