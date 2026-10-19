//! RF-1142: isolated real WebView2/Windows frame. No application setup, Vault or plugins.
#[cfg(target_os = "windows")]
mod regression {
    use serde_json::{json, Value};
    use solo_soul::commands::window::{self, TitlebarColor, TitlebarControlRect};
    use std::{
        borrow::Cow,
        path::PathBuf,
        sync::{Arc, Mutex},
    };
    use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};
    use windows::Win32::{
        Foundation::{LPARAM, POINT, RECT, WPARAM},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{
            GetClientRect, GetWindowLongPtrW, GetWindowRect, SendMessageW, GWL_STYLE, WM_NCHITTEST,
        },
    };

    const HTML: &str = r#"<!doctype html><html><head><meta charset="utf-8"><style>
    html,body{margin:0;height:100%;font:14px 'Segoe UI';color:#eee;background:transparent}
    header{height:52px;display:flex;align-items:center;gap:10px;padding-left:12px;box-sizing:border-box}
    button{height:32px;border:0;border-radius:6px;background:#ffffff20;color:inherit;padding:0 12px}
    main{background:#242424;margin:0 8px 8px;border-radius:12px;padding:24px;height:calc(100% - 60px);box-sizing:border-box}
    h1{font-size:22px}pre{white-space:pre-wrap}button:focus-visible{outline:2px solid #8bc2ff}
    </style></head><body><header data-tauri-drag-region><button data-tauri-drag-region="false" id="interact">Interact</button><button data-tauri-drag-region="false" id="theme">Light/Dark</button><button data-tauri-drag-region="false" id="disable">Disable/Enable</button></header>
    <main><h1>RF-1142 — isolated Windows frame</h1><p>No account, password, Vault, or application plugins are loaded.</p>
    <p>Native caption controls remain at the upper right. Test drag, resize, double-click, maximize, minimize and close.</p><pre id="status">Loading</pre></main><script>
    let dark=true,enabled=false,clicks=0;const invoke=(...a)=>window.__TAURI_INTERNALS__.invoke(...a);
    const regions=()=>Array.from(document.querySelectorAll('header button')).map(b=>{const r=b.getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height}});
    async function report(action){const result=await invoke('frame_action',{action,enabled,dark,clicks,regions:regions()});document.body.style.color=dark?'#eee':'#111';document.querySelector('main').style.background=dark?'#242424':'#fafaf8';document.querySelector('#status').textContent=JSON.stringify(result,null,2)}
    document.querySelector('#interact').onclick=()=>{clicks++;report('interact')};
    document.querySelector('#theme').onclick=()=>{dark=!dark;report('theme')};
    document.querySelector('#disable').onclick=()=>{enabled=!enabled;report('configure')};
    window.addEventListener('resize',()=>report('resize'));window.addEventListener('focus',()=>report('focus'));
    window.addEventListener('keydown',e=>{if(e.key==='F8')report('probe')});
    (async()=>{await invoke('set_titlebar_controls',{regions:[]});await report('legacy');enabled=true;await invoke('set_titlebar_controls',{regions:regions(),windowsFrame:true});await report('configure')})().catch(e=>document.querySelector('#status').textContent=String(e));
    </script></body></html>"#;

    struct Assets(String);
    impl tauri::Assets<tauri::Wry> for Assets {
        fn get(&self, key: &tauri::utils::assets::AssetKey) -> Option<Cow<'_, [u8]>> {
            (key.as_ref() == "/index.html").then_some(Cow::Borrowed(self.0.as_bytes()))
        }
        fn iter(&self) -> Box<tauri::utils::assets::AssetsIter<'_>> {
            Box::new(std::iter::once((
                "index.html".into(),
                self.0.as_bytes().into(),
            )))
        }
        fn csp_hashes(
            &self,
            _: &tauri::utils::assets::AssetKey,
        ) -> Box<dyn Iterator<Item = tauri::utils::assets::CspHash<'_>> + '_> {
            Box::new(std::iter::empty())
        }
    }
    struct Reports {
        output: PathBuf,
        sequence: Mutex<usize>,
    }
    fn record(reports: &Reports, value: Value) -> Result<(), String> {
        use std::io::Write;
        let mut sequence = reports.sequence.lock().map_err(|e| e.to_string())?;
        if *sequence >= 500 {
            return Err("Report bound exceeded".into());
        }
        let path = reports.output.join(format!("report-{:03}.json", *sequence));
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .and_then(|mut f| f.write_all(serde_json::to_string_pretty(&value)?.as_bytes()))
            .map_err(|e| e.to_string())?;
        *sequence += 1;
        Ok(())
    }
    #[tauri::command]
    async fn frame_action(
        window: WebviewWindow,
        action: String,
        enabled: bool,
        dark: bool,
        clicks: u32,
        regions: Vec<TitlebarControlRect>,
        reports: tauri::State<'_, Arc<Reports>>,
    ) -> Result<Value, String> {
        if ![
            "legacy",
            "interact",
            "configure",
            "theme",
            "resize",
            "focus",
            "probe",
        ]
        .contains(&action.as_str())
        {
            return Err("Unknown action".into());
        }
        if action == "configure" {
            window::set_titlebar_controls(window.clone(), regions, Some(enabled), None).await?;
        }
        let color = if dark {
            TitlebarColor {
                red: 28,
                green: 28,
                blue: 30,
            }
        } else {
            TitlebarColor {
                red: 250,
                green: 250,
                blue: 248,
            }
        };
        if action == "configure" || action == "theme" {
            let _ = window::set_titlebar_color(window.clone(), color).await?;
        }
        let layout = window::get_window_layout(window.clone()).await?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let target = window.clone();
        window.run_on_main_thread(move||{
            let result=(||->Result<Value,String>{
                let hwnd=target.hwnd().map_err(|e|e.to_string())?;let mut outer=RECT::default();let mut client=RECT::default();let mut origin=POINT::default();
                // SAFETY: Only this isolated owned HWND is inspected on its UI thread.
                unsafe{GetWindowRect(hwnd,&mut outer).map_err(|e|e.to_string())?;GetClientRect(hwnd,&mut client).map_err(|e|e.to_string())?;
                    if !ClientToScreen(hwnd,&mut origin).as_bool(){return Err("Client origin failed".into())}}
                let mut hits=Vec::new();
                for (name,x,y) in [("control",30,26),("blank",420,26),("bottom",420,client.bottom-1)] {
                    let packed=(((origin.y+y)as u16 as usize)<<16)|((origin.x+x)as u16 as usize);
                    let hit=unsafe{SendMessageW(hwnd,WM_NCHITTEST,Some(WPARAM(0)),Some(LPARAM(packed as isize)))}.0;
                    hits.push(json!({"name":name,"hit":hit}));
                }
                let backdrop = unsafe {
                    use windows::core::w;
                    use windows::Win32::UI::WindowsAndMessaging::*;
                    let handle = GetPropW(hwnd, w!("SoloSoul.CaptionBackdrop.v1"));
                    if handle.is_invalid() { Value::Null } else {
                        let layer = windows::Win32::Foundation::HWND(handle.0);
                        let mut rect = RECT::default();
                        GetWindowRect(layer, &mut rect).map_err(|e|e.to_string())?;
                        let rgba = GetPropW(layer, w!("SoloSoul.CaptionBackdrop.Rgba")).0 as usize;
                        let mut class = [0u16;64]; let length=GetClassNameW(layer,&mut class);
                        json!({"class":String::from_utf16_lossy(&class[..length.max(0) as usize]), "ownerVerified":GetParent(layer).ok()==Some(hwnd),
                            "rect":[rect.left,rect.top,rect.right,rect.bottom],"visible":IsWindowVisible(layer).as_bool(), "alpha":(rgba>>24)&255,
                            "rgb":rgba&0xffffff,"rendering":"singlePerPixelSurface","exStyle":GetWindowLongPtrW(layer,GWL_EXSTYLE)})
                    }
                };
                use windows::Win32::Graphics::Dwm::*;
                let read_attribute = |attribute| {
                    let mut value = 0u32;
                    // SAFETY: Only this owned HWND is read, with a correctly sized DWORD output.
                    match unsafe { DwmGetWindowAttribute(hwnd, attribute, (&mut value as *mut u32).cast(), 4) } {
                        Ok(()) => json!({"value":value}),
                        Err(error) => json!({"error":error.code().0}),
                    }
                };
                let caption_color = read_attribute(DWMWA_CAPTION_COLOR);
                let backdrop_type = read_attribute(DWMWA_SYSTEMBACKDROP_TYPE);
                let dark_mode = read_attribute(DWMWA_USE_IMMERSIVE_DARK_MODE);
                let native_theme = format!("{:?}", target.theme().map_err(|error|error.to_string())?);
                Ok(json!({"captionColor":caption_color,"systemBackdropType":backdrop_type,"immersiveDarkMode":dark_mode,"nativeTheme":native_theme,"style":unsafe{GetWindowLongPtrW(hwnd,GWL_STYLE)},"window":[outer.left,outer.top,outer.right,outer.bottom],"client":[origin.x,origin.y,client.right,client.bottom],"hits":hits,"captionBackdrop":backdrop}))
            })();let _=tx.send(result);
        }).map_err(|e|e.to_string())?;
        let native = rx.await.map_err(|e| e.to_string())??;
        let result = json!({"action":action,"enabled":enabled,"dark":dark,"clicks":clicks,"layout":layout,"native":native});
        record(&reports, result.clone())?;
        Ok(result)
    }
    // The public fixture avoids production setup, accounts and persisted window state.
    #[tauri::command]
    fn frame_frontend_report(
        report: Value,
        reports: tauri::State<'_, Arc<Reports>>,
    ) -> Result<(), String> {
        if report.to_string().len() > 65536 {
            return Err("Frontend report bound exceeded".into());
        }
        record(&reports, report)
    }
    #[tauri::command]
    fn show_main_window(window: WebviewWindow) -> Result<(), String> {
        window.show().map_err(|e| e.to_string())
    }
    pub fn run() -> Result<(), String> {
        let output = PathBuf::from(
            std::env::args()
                .nth(1)
                .ok_or("Expected new absolute output directory")?,
        );
        if !output.is_absolute() {
            return Err("Output must be absolute".into());
        }
        std::fs::create_dir(&output).map_err(|e| e.to_string())?;
        let reports = Arc::new(Reports {
            output: output.clone(),
            sequence: Mutex::new(0),
        });
        let fixture = std::env::args().nth(2);
        let html = if let Some(path) = &fixture {
            let html = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            if !html.starts_with("<!-- SoloSoul RF-1144 isolated public fixture -->") {
                return Err("Expected an RF-1144 public frontend fixture".into());
            }
            html
        } else {
            HTML.into()
        };
        let mut context = tauri::test::mock_context(Assets(html));
        context.config_mut().identifier = "com.solosoul.windows-frame-regression".into();
        context.config_mut().app.windows.clear();
        // mock_context has no core ACL. Permit only the two local commands used
        // by Tauri's standard drag-region script in this single-window fixture.
        for command in [
            "plugin:window|start_dragging",
            "plugin:window|internal_toggle_maximize",
            "plugin:event|listen",
            "plugin:event|unlisten",
        ] {
            context
                .runtime_authority_mut()
                .__allow_command(command.into(), tauri::utils::acl::ExecutionContext::Local);
        }
        tauri::Builder::default()
            .manage(reports.clone())
            .invoke_handler(tauri::generate_handler![
                frame_action,
                frame_frontend_report,
                window::set_titlebar_controls,
                window::get_window_layout,
                window::set_titlebar_color,
                show_main_window
            ])
            .setup(move |app| {
                let window = WebviewWindowBuilder::new(
                    app,
                    "frame-regression",
                    WebviewUrl::App("index.html".into()),
                )
                .title(if fixture.is_some() {
                    "RF-1144 — Windows frontend regression"
                } else {
                    "RF-1142 — Windows frame regression"
                })
                .inner_size(900.0, 600.0)
                .min_inner_size(400.0, 300.0)
                .decorations(true)
                .transparent(true)
                .visible(fixture.is_none())
                .data_directory(output.join("webview-profile"))
                .build()?;
                let events = reports.clone();
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    let name = match event {
                        tauri::WindowEvent::CloseRequested { .. } => "native-close",
                        tauri::WindowEvent::Destroyed => "destroyed",
                        tauri::WindowEvent::Moved(_) => "moved",
                        tauri::WindowEvent::Resized(_) => "resized",
                        _ => return,
                    };
                    if let Err(e) = record(&events, json!({"nativeEvent":name})) {
                        eprintln!("Report failed: {e}");
                        handle.exit(1)
                    }
                });
                let deadline_reports = reports.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(900));
                    let _ = record(
                        &deadline_reports,
                        json!({"failure":"native-validation-timeout"}),
                    );
                    // Mock contexts can return 0 from app.exit. Fail explicitly
                    // without synthesizing a successful native-close event.
                    std::process::exit(2)
                });
                Ok(())
            })
            .run(context)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = regression::run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1)
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("Windows native frame regression requires Windows");
        std::process::exit(1)
    }
}
