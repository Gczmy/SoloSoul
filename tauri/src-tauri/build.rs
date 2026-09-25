// build.rs 中 panic 是预期行为：构建脚本应在依赖缺失时失败，
// 因此允许 unwrap/expect 不传播为 Result。
#![allow(clippy::unwrap_used, clippy::expect_used)]

fn main() {
    // 内联移动插件的事件监听同样需要 ACL；只开放监听，菜单命令仍经 Rust 校验。
    let attributes = tauri_build::Attributes::new().plugin(
        "android-glass",
        tauri_build::InlinedPlugin::new().commands(&["register_listener", "remove_listener"]),
    );
    let attributes = if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // RF-901：Tauri 的资源默认只链接 bins，库测试缺少 Common Controls v6
        // 清单会在加载 TaskDialogIndirect 时退出。由链接器为所有可执行目标嵌入，
        // 同时关闭资源中的重复清单；应用图标和版本仍由 Tauri 生成。
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
        attributes.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
    } else {
        attributes
    };
    tauri_build::try_build(attributes).expect("failed to build Tauri application");
    generate_app_level_names();
}

/// 从 app_level_names.json 生成 Rust 常量与 Kotlin 常量文件，
/// 保证两端 APP_LEVEL_NAMES 单一来源、避免手动同步。
fn generate_app_level_names() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .expect("CARGO_MANIFEST_DIR not set");
    let out_dir = std::env::var("OUT_DIR")
        .map(std::path::PathBuf::from)
        .expect("OUT_DIR not set");

    let json_path = manifest_dir.join("app_level_names.json");
    println!("cargo:rerun-if-changed={}", json_path.display());

    let json_str =
        std::fs::read_to_string(&json_path).expect("failed to read app_level_names.json");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("failed to parse app_level_names.json");
    let names = json
        .get("names")
        .and_then(|v| v.as_array())
        .expect("app_level_names.json must contain a 'names' array");

    // 生成 Rust 常量到 OUT_DIR（Kotlin 端由 Gradle 任务生成，遵循 Cargo OUT_DIR 约定）
    let rust_items = names
        .iter()
        .map(|v| format!("    \"{}\"", v.as_str().expect("names must be strings")))
        .collect::<Vec<_>>()
        .join(",\n");
    let rust_content = format!(
        "/// Auto-generated from app_level_names.json. Do not edit manually.\n\
         pub const APP_LEVEL_NAMES: &[&str] = &[\n{}\n];\n",
        rust_items
    );
    std::fs::write(out_dir.join("app_level_names.rs"), rust_content)
        .expect("failed to write app_level_names.rs");
}
