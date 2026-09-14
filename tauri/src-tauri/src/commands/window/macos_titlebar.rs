//! 顶部空白带使用原生事件，保留后台首次拖拽；不改变 WKWebView 的首次点击策略。
use super::super::TitlebarControlRect;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::NSObjectProtocol, sel, DefinedClass,
    MainThreadOnly,
};
use objc2_app_kit::{
    NSEvent, NSLayoutAttribute, NSLayoutConstraint, NSLayoutRelation, NSTitlebarSeparatorStyle,
    NSToolbar, NSToolbarDisplayMode, NSView, NSWindow, NSWindowButton, NSWindowToolbarStyle,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSArray, NSPoint, NSUserDefaults};
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub struct TitlebarIvars {
    double_click_origin: Cell<Option<NSPoint>>,
    controls: RefCell<Vec<TitlebarControlRect>>,
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

        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
            self.hit_test_impl(point)
        }

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

impl TitlebarDragView {
    fn hit_test_impl(&self, point: NSPoint) -> Option<Retained<NSView>> {
        // hitTest 的点位属于父视图；网页原点位于左上角，AppKit 通常位于左下角。
        // 控件区域穿透至 WKWebView，其他空白仍由同一条原生拖拽带处理。
        let root = unsafe { self.superview() }?;
        let bounds = root.bounds();
        let x = point.x - bounds.origin.x;
        let y = if root.isFlipped() {
            point.y - bounds.origin.y
        } else {
            bounds.origin.y + bounds.size.height - point.y
        };
        if self
            .ivars()
            .controls
            .borrow()
            .iter()
            .any(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
        {
            return None;
        }
        // SAFETY: 交由 NSView 的默认命中测试处理边界，返回的对象受窗口持有。
        unsafe { msg_send![super(self), hitTest: point] }
    }
}

pub fn height(window: &NSWindow) -> f64 {
    let Some(root) = window.contentView() else {
        return 0.0;
    };
    let bounds = root.bounds();
    let layout = root.convertRect_fromView(window.contentLayoutRect(), None);
    (bounds.origin.y + bounds.size.height - layout.origin.y - layout.size.height)
        .clamp(0.0, bounds.size.height)
}

pub fn traffic_lights_right(window: &NSWindow) -> f64 {
    if height(window) == 0.0 {
        return 0.0;
    }
    let Some(root) = window.contentView() else {
        return 0.0;
    };
    [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .into_iter()
    .filter_map(|kind| window.standardWindowButton(kind))
    .filter(|button| !button.isHiddenOrHasHiddenAncestor())
    .map(|button| {
        let frame = root.convertRect_fromView(button.bounds(), Some(&button));
        frame.origin.x + frame.size.width - root.bounds().origin.x
    })
    .fold(0.0, f64::max)
}

pub fn set_controls(
    native: tauri::webview::PlatformWebview,
    regions: Vec<TitlebarControlRect>,
) -> Result<(), String> {
    // SAFETY: Tauri 的 with_webview 在主线程提供存活的 NSWindow。
    let window = unsafe { &*native.ns_window().cast::<NSWindow>() };
    let root = window
        .contentView()
        .ok_or("Window content view unavailable")?;
    let view = root
        .viewWithTag(0x53535442)
        .ok_or("Titlebar drag view unavailable")?;
    let view = view
        .downcast::<TitlebarDragView>()
        .map_err(|_| "Unexpected titlebar view")?;
    *view.ivars().controls.borrow_mut() = regions;
    Ok(())
}

pub fn install(window: &NSWindow, root: &NSView) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Titlebar installation requires main thread")?;
    // 让 AppKit 同时增加栏高并居中系统交通灯；不手动改按钮或私有标题栏容器。
    // IconOnly 避免系统为不存在的图标标签额外预留第二行，本机单行高度为 52pt。
    let toolbar =
        NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), ns_string!("SoloSoulTitlebar"));
    toolbar.setDisplayMode(NSToolbarDisplayMode::IconOnly);
    toolbar.setAllowsUserCustomization(false);
    // ToolbarStyle / SeparatorStyle 为 macOS 11+ API，旧系统沿用系统默认工具栏。
    if window.respondsToSelector(sel!(setToolbarStyle:)) {
        window.setToolbarStyle(NSWindowToolbarStyle::Unified);
    }
    if window.respondsToSelector(sel!(setTitlebarSeparatorStyle:)) {
        window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
    }
    window.setToolbar(Some(&toolbar));
    let guide = window
        .contentLayoutGuide()
        .ok_or("Window content layout guide unavailable")?;
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
