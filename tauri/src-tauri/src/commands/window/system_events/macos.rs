use objc2::{
    define_class, msg_send, rc::Retained, runtime::AnyObject, sel, ClassType, DefinedClass,
    MainThreadOnly,
};
use objc2_app_kit::{
    NSApplication, NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions,
    NSNotification, NSNotificationCenter, NSObject, NSObjectNSKeyValueObserverRegistration,
    NSString,
};
use std::{cell::RefCell, ffi::c_void};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SoloSoulSystemAppearanceObserver"]
    #[ivars = tauri::AppHandle]
    struct Observer;

    impl Observer {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn appearance_changed(
            &self,
            _key: Option<&NSString>,
            _object: Option<&AnyObject>,
            _change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            _context: *mut c_void,
        ) {
            super::notify(self.ivars());
        }

        #[unsafe(method(solosoulAccessibilityChanged:))]
        fn accessibility_changed(&self, _notification: &NSNotification) {
            super::notify(self.ivars());
        }
    }
);

struct Observation {
    application: Retained<NSApplication>,
    observer: Retained<Observer>,
    center: Retained<NSNotificationCenter>,
}

impl Drop for Observation {
    fn drop(&mut self) {
        // SAFETY: 仅在注册的主线程释放；观察对象和 observer 均由本结构强引用。
        unsafe {
            self.application
                .as_super()
                .as_super()
                .removeObserver_forKeyPath(
                    self.observer.as_super(),
                    ns_string!("effectiveAppearance"),
                );
            self.center.removeObserver(&self.observer);
        }
    }
}

thread_local! {
    static OBSERVATION: RefCell<Option<Observation>> = const { RefCell::new(None) };
}

pub(super) fn install(app: &tauri::AppHandle) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Appearance observer requires the main thread")?;
    OBSERVATION.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let application = NSApplication::sharedApplication(mtm);
        let allocated = Observer::alloc(mtm).set_ivars(app.clone());
        // SAFETY: 标准 NSObject 初始化；本线程保留 observer 到注销完成。
        let observer: Retained<Observer> = unsafe { msg_send![super(allocated), init] };
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // 窗口的显式 Aqua/DarkAqua 不改变 NSApp.effectiveAppearance，故手动模式仍能
        // 观察真实系统变化；不依赖未公开的 AppleInterfaceThemeChangedNotification 名称。
        // SAFETY: KVO 只注册该 selector/key，通知 selector 接收 NSNotification；
        // AppKit 在主线程更新外观，注销与注册成对。
        unsafe {
            application
                .as_super()
                .as_super()
                .addObserver_forKeyPath_options_context(
                    observer.as_super(),
                    ns_string!("effectiveAppearance"),
                    NSKeyValueObservingOptions::New,
                    std::ptr::null_mut(),
                );
            center.addObserver_selector_name_object(
                &observer,
                sel!(solosoulAccessibilityChanged:),
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
            );
        }
        *slot.borrow_mut() = Some(Observation {
            application,
            observer,
            center,
        });
    });
    Ok(())
}

pub(super) fn uninstall() {
    OBSERVATION.with(|slot| drop(slot.borrow_mut().take()));
}
