//! iOS 前端自行通过 safe-area-inset 与可视视口避让系统栏和键盘。
//! 禁止 WKWebView 再扣一次原生安全区；不替换滚动 delegate 或键盘事件。

use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_ui_kit::{UIScrollView, UIScrollViewContentInsetAdjustmentBehavior};

pub(super) fn prepare(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.with_webview(|native| {
        // SAFETY: with_webview 在主线程提供存活的 WKWebView，仅在回调内借用。
        let Some(webview) = (unsafe { native.inner().cast::<AnyObject>().as_ref() }) else {
            return;
        };
        // SAFETY: WKWebView 的公开 scrollView 属性返回其持有的 UIScrollView。
        let scroll: Retained<UIScrollView> = unsafe { msg_send![webview, scrollView] };
        scroll.setContentInsetAdjustmentBehavior(UIScrollViewContentInsetAdjustmentBehavior::Never);
    })
}
