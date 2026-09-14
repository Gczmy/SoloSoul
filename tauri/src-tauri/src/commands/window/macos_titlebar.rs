//! 顶部空白带使用原生事件，保留后台首次拖拽；不改变 WKWebView 的首次点击策略。
use objc2::{define_class, msg_send, rc::Retained, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSEvent, NSLayoutAttribute, NSLayoutConstraint, NSLayoutRelation, NSView, NSWindow,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSArray, NSPoint, NSUserDefaults};
use std::cell::Cell;

#[derive(Default)]
pub struct TitlebarIvars {
    double_click_origin: Cell<Option<NSPoint>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "SoloSoulTitlebarDragView"]
    #[ivars = TitlebarIvars]
    struct TitlebarDragView;

    impl TitlebarDragView {
        #[unsafe(method(tag))]
        fn tag(&self) -> isize { 0x53535442 }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if event.clickCount() == 2 {
                self.ivars().double_click_origin.set(Some(event.locationInWindow()));
            } else if let Some(window) = self.window() {
                self.ivars().double_click_origin.set(None);
                window.performWindowDragWithEvent(event);
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.ivars().double_click_origin.set(None);
            if let Some(window) = self.window() {
                window.performWindowDragWithEvent(event);
            }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            // 和系统标题栏一样在抬起时执行双击；移动过的第二次按下不触发缩放。
            if event.clickCount() != 2
                || self.ivars().double_click_origin.take() != Some(event.locationInWindow()) {
                return;
            }
            let Some(window) = self.window() else { return; };
            let action = NSUserDefaults::standardUserDefaults()
                .stringForKey(ns_string!("AppleActionOnDoubleClick"))
                .map(|value| value.to_string());
            match action.as_deref() {
                Some("Minimize") => window.performMiniaturize(None),
                Some("None") => {},
                _ => window.performZoom(None),
            }
        }
    }
);

pub fn height(window: &NSWindow) -> f64 {
    let Some(root) = window.contentView() else {
        return 0.0;
    };
    let bounds = root.bounds();
    let layout = root.convertRect_fromView(window.contentLayoutRect(), None);
    (bounds.origin.y + bounds.size.height - layout.origin.y - layout.size.height)
        .clamp(0.0, bounds.size.height)
}

pub fn install(window: &NSWindow, root: &NSView) -> Result<(), String> {
    let guide = window
        .contentLayoutGuide()
        .ok_or("Window content layout guide unavailable")?;
    let mtm = MainThreadMarker::new().ok_or("Titlebar installation requires main thread")?;
    let allocated = TitlebarDragView::alloc(mtm).set_ivars(TitlebarIvars::default());
    // SAFETY: NSView 的标准 init，返回值由 root 强引用管理。
    let view: Retained<TitlebarDragView> = unsafe { msg_send![super(allocated), init] };
    root.addSubview(&view);
    view.setTranslatesAutoresizingMaskIntoConstraints(false);
    let constraints = [NSLayoutAttribute::Left, NSLayoutAttribute::Right,
        NSLayoutAttribute::Top, NSLayoutAttribute::Bottom].map(|edge| {
        let (target, target_edge) = if edge == NSLayoutAttribute::Bottom {
            (&*guide, NSLayoutAttribute::Top)
        } else {
            (root.as_ref(), edge)
        };
        // SAFETY: 视图与布局指南均属于同一窗口。透明视图只占标题栏空白带。
        unsafe {
            NSLayoutConstraint::constraintWithItem_attribute_relatedBy_toItem_attribute_multiplier_constant(
                &view, edge, NSLayoutRelation::Equal, Some(target), target_edge, 1.0, 0.0)
        }
    });
    NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(&constraints));
    Ok(())
}
