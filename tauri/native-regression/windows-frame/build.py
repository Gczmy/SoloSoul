"""Build the production window module in an isolated, small Tauri test executable.

No production dependencies/lockfile are rewritten. This avoids linking OCR/Vault
for each native frame iteration. Existing workspace example remains available.
"""
from pathlib import Path
import hashlib,json,os,subprocess,sys

root=Path(__file__).resolve().parents[3]
output=Path(sys.argv[1])
if not output.is_absolute() or output.exists():
    raise SystemExit('Expected a new absolute output directory')
output.mkdir()
source=root/'tauri/src-tauri/examples/windows_frame_regression.rs'
original=source.read_text(encoding='utf-8')
old='use solo_soul::commands::window::{self, TitlebarColor, TitlebarControlRect};'
assert original.count(old)==1
module=root/'tauri/src-tauri/src/commands/window.rs'
main='#![allow(dead_code)]\n#[cfg(target_os="windows")]\n#[path='+json.dumps(module.as_posix())+']\nmod window;\n'+original.replace(old,'use crate::window::{self, TitlebarColor, TitlebarControlRect};').replace('//! RF-1142:', '// RF-1142:',1)
(output/'main.rs').write_text(main,encoding='utf-8')
(output/'Cargo.toml').write_text('''[package]
name="windows-frame-regression"
version="0.1.0"
edition="2021"
[workspace]
[[bin]]
name="windows-frame-regression"
path="main.rs"
[dependencies]
tauri={version="=2.11.2",features=["test","devtools","custom-protocol","macos-private-api"]}
tauri-plugin-window-state="=2.4.1"
window-vibrancy="=0.8.0"
windows={version="=0.61.3",features=["Win32_Foundation","Win32_Graphics_Dwm","Win32_Graphics_Gdi","Win32_UI_Controls","Win32_UI_HiDpi","Win32_UI_WindowsAndMessaging","Win32_UI_Shell","Win32_System_Registry","Win32_UI_Accessibility"]}
serde={version="1",features=["derive"]}
serde_json="1"
tokio={version="1",features=["full"]}
[profile.dev]
opt-level=1
strip="debuginfo"
''',encoding='utf-8')
(output/'Cargo.lock').write_bytes((root/'tauri/Cargo.lock').read_bytes())
(output/'build.rs').write_text('''fn main() {
 println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
 println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
}
''',encoding='utf-8')
inputs=[source,module,*module.parent.joinpath('window').glob('*.rs'),root/'tauri/src-tauri/Cargo.toml',root/'tauri/Cargo.lock']
freeze={p.relative_to(root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
(output/'source-manifest.json').write_text(json.dumps(freeze,indent=2),encoding='utf-8')
command=['cargo','build','--offline','--manifest-path',str(output/'Cargo.toml')]
code=subprocess.call(command)
if code==0:
 assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==sha for p,sha in freeze.items()),'Inputs changed during native build'
raise SystemExit(code)
