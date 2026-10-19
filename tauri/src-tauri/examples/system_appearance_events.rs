//! RF-1192: real HWND notifications and Tauri events, with process-local HKCU fixtures.
//! Does not change the user's Windows theme, create accounts or open a Vault.
#[cfg(target_os = "windows")]
mod regression {
    use solo_soul::commands::{
        system::get_system_theme,
        window::{self, observe_system_appearance, TitlebarColor},
    };
    use std::{
        borrow::Cow,
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use tauri::{Listener, WebviewUrl, WebviewWindowBuilder};
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{LPARAM, WPARAM},
            Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE},
            System::Registry::{
                RegCloseKey, RegCreateKeyW, RegDeleteTreeW, RegOverridePredefKey, RegSetValueExW,
                HKEY, HKEY_CURRENT_USER, REG_DWORD,
            },
            UI::WindowsAndMessaging::{
                SendMessageW, WM_SETTINGCHANGE, WM_SYSCOLORCHANGE, WM_THEMECHANGED,
            },
        },
    };

    struct Assets;
    impl tauri::Assets<tauri::Wry> for Assets {
        fn get(&self, _: &tauri::utils::assets::AssetKey) -> Option<Cow<'_, [u8]>> {
            Some(Cow::Borrowed(
                b"<!doctype html><title>RF-1192 system appearance events</title>",
            ))
        }
        fn iter(&self) -> Box<tauri::utils::assets::AssetsIter<'_>> {
            Box::new(std::iter::empty())
        }
        fn csp_hashes(
            &self,
            _: &tauri::utils::assets::AssetKey,
        ) -> Box<dyn Iterator<Item = tauri::utils::assets::CspHash<'_>> + '_> {
            Box::new(std::iter::empty())
        }
    }

    struct RegistryFixture {
        path: Vec<u16>,
        root: HKEY,
        settings: HKEY,
    }
    impl RegistryFixture {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let path = format!("Software\\SoloSoul-rf1192-{}", uuid::Uuid::new_v4())
                .encode_utf16()
                .chain([0])
                .collect::<Vec<_>>();
            let mut fixture = Self {
                path,
                root: HKEY::default(),
                settings: HKEY::default(),
            };
            // SAFETY: only creates a fresh UUID-scoped test subtree under current user.
            unsafe {
                RegCreateKeyW(
                    HKEY_CURRENT_USER,
                    PCWSTR(fixture.path.as_ptr()),
                    &mut fixture.root,
                )
                .ok()?;
                RegCreateKeyW(
                    fixture.root,
                    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                    &mut fixture.settings,
                )
                .ok()?;
            }
            Ok(fixture)
        }
        fn with_mode(
            &self,
            mode: &str,
            callback: impl FnOnce(),
        ) -> Result<(), Box<dyn std::error::Error>> {
            let light: u32 = u32::from(mode == "light");
            // SAFETY: HKCU redirection is process-local, restored before returning; no user
            // personalization key is ever written. Registry handles outlive this callback.
            unsafe {
                RegSetValueExW(
                    self.settings,
                    w!("AppsUseLightTheme"),
                    None,
                    REG_DWORD,
                    Some(&light.to_le_bytes()),
                )
                .ok()?;
                RegOverridePredefKey(HKEY_CURRENT_USER, Some(self.root)).ok()?;
            }
            struct Restore;
            impl Drop for Restore {
                fn drop(&mut self) {
                    // SAFETY: restoring the predefined key does not close the retained test handle.
                    unsafe {
                        let _ = RegOverridePredefKey(HKEY_CURRENT_USER, None);
                    }
                }
            }
            let restore = Restore;
            callback();
            unsafe {
                RegOverridePredefKey(HKEY_CURRENT_USER, None).ok()?;
            }
            drop(restore);
            Ok(())
        }
    }
    impl Drop for RegistryFixture {
        fn drop(&mut self) {
            // SAFETY: fixed prefix plus fresh UUID; never deletes a user personalization key.
            unsafe {
                let _ = RegOverridePredefKey(HKEY_CURRENT_USER, None);
                let _ = RegCloseKey(self.settings);
                let _ = RegCloseKey(self.root);
                let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(self.path.as_ptr()));
            }
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let output = PathBuf::from(
            std::env::args()
                .nth(1)
                .ok_or("Expected new output directory")?,
        );
        if !output.is_absolute() {
            return Err("Output must be absolute".into());
        }
        std::fs::create_dir(&output)?;
        let mut context = tauri::test::mock_context(Assets);
        context.config_mut().identifier = "com.solosoul.system-appearance-regression".into();
        context.config_mut().app.windows.clear();
        tauri::Builder::default().setup(move |app| {
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("RF-1192 — System appearance events")
                .visible(false).data_directory(output.join("webview-profile")).build()?;
            let original = get_system_theme()?;
            // Explicit window mode must not suppress independent system notifications.
            window.set_theme(Some(if original == "dark" { tauri::Theme::Light } else { tauri::Theme::Dark }))?;
            let events = Arc::new(Mutex::new(Vec::<String>::new()));
            let received = events.clone();
            app.listen("system-theme-changed", move |event| {
                received.lock().unwrap().push(serde_json::from_str(event.payload()).unwrap());
            });
            observe_system_appearance(app.handle())?;
            observe_system_appearance(app.handle())?;
            events.lock().unwrap().clear();
            let fixture = RegistryFixture::new()?;
            let hwnd = window.hwnd()?;
            let opposite = if original == "dark" { "light" } else { "dark" };
            for mode in [opposite, original.as_str()] {
                fixture.with_mode(mode, || {
                    assert_eq!(get_system_theme().unwrap(), mode);
                    // SAFETY: the real retained HWND is on this thread; lParam is null.
                    // These targeted synthetic messages test production dispatch without
                    // broadcasting settings changes to other applications.
                    unsafe {
                        for message in [WM_SETTINGCHANGE, WM_SETTINGCHANGE, WM_THEMECHANGED, WM_SYSCOLORCHANGE] {
                            SendMessageW(hwnd, message, Some(WPARAM(0)), Some(LPARAM(0)));
                        }
                    }
                })?;
            }
            assert_eq!(*events.lock().unwrap(), vec![opposite.to_string(), original.clone()]);
            assert_eq!(get_system_theme()?, original);
            drop(fixture);
            let handle = app.handle().clone();
            let expected = vec![opposite.to_string(), original];
            tauri::async_runtime::spawn(async move {
                let mut native_updates = 0;
                let validation: Result<(), String> = async {
                    for dark in [false, true] {
                        let tone = if dark {24} else {243};
                        window::set_titlebar_color(window.clone(), TitlebarColor {red:tone,green:tone,blue:tone}).await?;
                        let theme = window.theme().map_err(|e| e.to_string())?;
                        if theme != if dark {tauri::Theme::Dark} else {tauri::Theme::Light} {
                            return Err("Native window theme disagrees with resolved color".into());
                        }
                        let mut actual = 0u32;
                        // SAFETY: live retained HWND, writable DWORD output of the documented size.
                        unsafe {
                            DwmGetWindowAttribute(window.hwnd().map_err(|e| e.to_string())?,
                                DWMWA_USE_IMMERSIVE_DARK_MODE, (&mut actual as *mut u32).cast(),
                                std::mem::size_of::<u32>() as u32).map_err(|e| e.to_string())?;
                        }
                        if actual != u32::from(dark) {return Err("DWM appearance disagrees with resolved color".into());}
                        native_updates += 1;
                    }
                    if *events.lock().unwrap() != expected { return Err("Window overrides generated false system events".into()); }
                    Ok(())
                }.await;
                let report = serde_json::json!({
                    "task":"RF-1192", "passed":validation.is_ok(),
                    "native":"real HWND subclass, registry reads, Tauri event bus and DWM readback",
                    "messages":8,"changes":2,"duplicatesSuppressed":6,"nativeAppearanceUpdates":native_updates,
                    "explicitWindowTheme":true,"idempotentInstall":true,"userThemeRestored":true,
                    "error":validation.err(),
                    "limitations":["Synthetic native messages with process-local registry fixtures; no Windows settings UI or visual assertion."]
                });
                let passed = report["passed"] == true;
                match std::fs::write(output.join("result.json"), serde_json::to_vec_pretty(&report).unwrap()) {
                    Ok(()) => handle.exit(if passed {0} else {1}),
                    Err(error) => {eprintln!("Cannot save native result: {error}");handle.exit(1);}
                }
            });
            Ok(())
        }).run(context)?;
        Ok(())
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = regression::run() {
        eprintln!("RF-1192 failed: {error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "windows"))]
    eprintln!("Windows-only native notification regression");
}
