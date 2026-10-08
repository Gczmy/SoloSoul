//! 顶部空白带使用原生事件，保留后台首次拖拽；不改变 WKWebView 的首次点击策略。
use super::super::TitlebarControlRect;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::NSObjectProtocol, sel, DefinedClass,
    MainThreadOnly,
};
use objc2_app_kit::{
    NSEvent, NSEventType, NSLayoutAttribute, NSLayoutConstraint, NSLayoutRelation,
    NSTitlebarSeparatorStyle, NSToolbar, NSToolbarDisplayMode, NSView, NSWindow, NSWindowButton,
    NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    NSWindowStyleMask, NSWindowToolbarStyle, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSArray, NSNotification, NSNotificationCenter, NSObject,
    NSObjectNSDelayedPerforming, NSPoint, NSUserDefaults,
};
use std::cell::{Cell, RefCell};
use tauri::Emitter;

#[derive(Default)]
pub struct TitlebarIvars {
    double_click_origin: Cell<Option<NSPoint>>,
    controls: RefCell<Vec<TitlebarControlRect>>,
    toolbar: RefCell<Option<Retained<NSToolbar>>>,
    fullscreen: Cell<bool>,
    event_target: RefCell<Option<(tauri::AppHandle, String)>>,
}

fn is_pointer_tracking_event(event_type: NSEventType) -> bool {
    matches!(
        event_type,
        NSEventType::MouseMoved
            | NSEventType::MouseEntered
            | NSEventType::MouseExited
            | NSEventType::CursorUpdate
    )
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

        #[unsafe(method(solosoulWillEnterFullscreen:))]
        fn will_enter_fullscreen(&self, _notification: &NSNotification) {
            self.ivars().fullscreen.set(true);
            // 必须在系统生成全屏动画/浮动工具栏之前移除，DidEnter 时处理已太晚。
            self.set_toolbar_visible(false);
            self.notify_layout();
        }

        #[unsafe(method(solosoulDidEnterFullscreen:))]
        fn did_enter_fullscreen(&self, _notification: &NSNotification) {
            self.ivars().fullscreen.set(true);
            self.set_toolbar_visible(false);
            self.notify_layout();
        }

        #[unsafe(method(solosoulWillExitFullscreen:))]
        fn will_exit_fullscreen(&self, _notification: &NSNotification) {
            // 退出动画期间仍不添加空浮动工具栏；完成后才恢复窗口模式工具栏。
            self.ivars().fullscreen.set(true);
        }

        #[unsafe(method(solosoulDidExitFullscreen:))]
        fn did_exit_fullscreen(&self, _notification: &NSNotification) {
            self.ivars().fullscreen.set(false);
            self.set_toolbar_visible(true);
            // AppKit 在通知栈返回后还会恢复工具栏/窗口几何。下一轮主运行循环
            // 再校正可见性并发布最终几何，不能靠提前到达的 resize 通知收尾。
            // SAFETY: selector 接收一个可空 NSObject；0 表示下一轮默认运行循环，
            // 不使用猜测的动画时长，也不循环轮询或重建材质。
            unsafe {
                self.performSelector_withObject_afterDelay(
                    sel!(solosoulFinalizeWindowedLayout:), None, 0.0);
            }
        }

        #[unsafe(method(solosoulFinalizeWindowedLayout:))]
        fn finalize_windowed_layout(&self, _object: Option<&NSObject>) {
            // 快速再次进入全屏时，上一轮延迟收尾不能重新挂回工具栏。
            if self.ivars().fullscreen.get() { return; }
            let Some(window) = self.window() else { return; };
            if window.styleMask().contains(NSWindowStyleMask::FullScreen) { return; }
            self.set_toolbar_visible(true);
            self.notify_layout();
        }

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
    fn notify_layout(&self) {
        if let Some((app, label)) = self.ivars().event_target.borrow().as_ref() {
            // 只通知所属窗口重新读取几何，不触发主题/材质同步，也不携带过期测量。
            let _ = app.emit_to(label.as_str(), "native-window-layout-changed", ());
        }
    }

    fn set_toolbar_visible(&self, visible: bool) {
        let Some(window) = self.window() else {
            return;
        };
        let toolbar = self.ivars().toolbar.borrow();
        let Some(toolbar) = toolbar.as_deref() else {
            return;
        };
        // 空工具栏只用于窗口模式的交通灯高度。全屏时 AppKit 将它移到独立
        // 顶部浮层，该浮层没有主窗口的 WKWebView/玻璃背景，可能遮住网页并显黑。
        // 进入动画前移除，退出完成后恢复同一实例；不替换 Tao 的窗口 delegate，
        // 不改变窗口背景或 WebView 层级。
        let desired = visible.then_some(toolbar);
        let mut changed = false;
        if window.toolbar().as_deref() != desired {
            window.setToolbar(desired);
            changed = true;
        }
        // AppKit 会在全屏时改变工具栏可见性；仅重新挂回实例会保留隐藏状态，
        // 将 52pt 顶栏降成普通 32pt 标题栏。退出完成后也校正此状态。
        if visible && !toolbar.isVisible() {
            toolbar.setVisible(true);
            changed = true;
        }
        if changed {
            if let Some(root) = window.contentView() {
                root.layoutSubtreeIfNeeded();
            }
        }
    }

    fn hit_test_impl(&self, point: NSPoint) -> Option<Retained<NSView>> {
        // WKWebView 的原生跟踪区只处理命中自身视图树的悬停事件。
        // 空白拖拽带覆盖网页时，纯移动/进出/光标更新必须穿透，才能按真实点位
        // 更新 CSS :hover；按下、拖动和抬起仍由本视图处理首次拖拽与双击。
        // currentEvent 是窗口正在分发的真实 NSEvent，不创建事件或调用 WebKit 私有接口。
        if self
            .window()
            .and_then(|window| window.currentEvent())
            .is_some_and(|event| is_pointer_tracking_event(event.r#type()))
        {
            return None;
        }
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

pub fn install(
    window: &NSWindow,
    root: &NSView,
    app: tauri::AppHandle,
    label: String,
) -> Result<(), String> {
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
    *view.ivars().toolbar.borrow_mut() = Some(toolbar);
    *view.ivars().event_target.borrow_mut() = Some((app, label));
    root.addSubview(&view);
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: AppKit 全屏通知在主线程发送；selector 的参数是 NSNotification。
    // observer 是窗口持有的 NSView。macOS 10.11+ 的 selector observer 使用弱引用，
    // 视图销毁时自动注销；不会通过通知中心保留窗口/视图形成循环。
    unsafe {
        for (name, selector) in [
            (
                NSWindowWillEnterFullScreenNotification,
                sel!(solosoulWillEnterFullscreen:),
            ),
            (
                NSWindowDidEnterFullScreenNotification,
                sel!(solosoulDidEnterFullscreen:),
            ),
            (
                NSWindowWillExitFullScreenNotification,
                sel!(solosoulWillExitFullscreen:),
            ),
            (
                NSWindowDidExitFullScreenNotification,
                sel!(solosoulDidExitFullscreen:),
            ),
        ] {
            center.addObserver_selector_name_object(&view, selector, Some(name), Some(window));
        }
    }
    let fullscreen = window.styleMask().contains(NSWindowStyleMask::FullScreen);
    view.ivars().fullscreen.set(fullscreen);
    view.set_toolbar_visible(!fullscreen);
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

#[cfg(test)]
mod tests {
    use super::is_pointer_tracking_event;
    use objc2_app_kit::NSEventType;

    #[test]
    fn pointer_tracking_events_pass_through_titlebar() {
        for event_type in [
            NSEventType::MouseMoved,
            NSEventType::MouseEntered,
            NSEventType::MouseExited,
            NSEventType::CursorUpdate,
        ] {
            assert!(is_pointer_tracking_event(event_type), "{event_type:?}");
        }
    }

    #[test]
    fn mouse_buttons_and_dragging_keep_native_titlebar_target() {
        // 双击与单击具有相同的 Down/Up 类型；点击次数不参与穿透判定，
        // 仍由 mouse_down/mouse_up 读取 clickCount 保留原生双击动作。
        for event_type in [
            NSEventType::LeftMouseDown,
            NSEventType::LeftMouseDragged,
            NSEventType::LeftMouseUp,
            NSEventType::RightMouseDown,
            NSEventType::RightMouseDragged,
            NSEventType::RightMouseUp,
            NSEventType::OtherMouseDown,
            NSEventType::OtherMouseDragged,
            NSEventType::OtherMouseUp,
            NSEventType::ScrollWheel,
            NSEventType::KeyDown,
            NSEventType::KeyUp,
        ] {
            assert!(!is_pointer_tracking_event(event_type), "{event_type:?}");
        }
    }
}
