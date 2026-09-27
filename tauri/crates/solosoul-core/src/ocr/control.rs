//! 可跨平台共享的 OCR 协作式取消信号；不包含宿主会话或事件依赖。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const OCR_CANCELLED: &str = "__OCR_CANCELLED__";

/// 取消只在阶段边界生效，不中断正在执行的原生推理或释放其资源。
#[derive(Clone, Default)]
pub struct OcrCancellation {
    cancelled: Arc<AtomicBool>,
}

impl OcrCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(OCR_CANCELLED.to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rf029_cancel_is_shared_across_threads_and_idempotent() {
        let cancellation = OcrCancellation::default();
        assert!(!cancellation.is_cancelled());
        assert_eq!(cancellation.check(), Ok(()));
        let worker = cancellation.clone();
        std::thread::spawn(move || worker.cancel()).join().unwrap();
        assert!(cancellation.is_cancelled());
        cancellation.cancel();
        assert_eq!(cancellation.check(), Err(OCR_CANCELLED.to_string()));
        assert_eq!(OcrCancellation::default().check(), Ok(()));
    }
}
