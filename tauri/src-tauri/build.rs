fn main() {
    tauri_build::build();
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

    // 生成 Rust 常量到 OUT_DIR
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

    // 生成 Kotlin 常量文件到 Android 源码目录
    let kt_names = names
        .iter()
        .map(|v| format!("        \"{}\"", v.as_str().expect("names must be strings")))
        .collect::<Vec<_>>()
        .join(",\n");
    let kt_content = format!(
        "package com.solosoul.app\n\n\
         /**\n \
         * Auto-generated from app_level_names.json. Do not edit manually.\n \
         */\n\
         object AppLevelNames {{\n    val NAMES = setOf(\n{}\n    )\n}}\n",
        kt_names
    );

    let kt_path =
        manifest_dir.join("gen/android/app/src/main/java/com/solosoul/app/AppLevelNames.kt");
    if let Some(parent) = kt_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create Kotlin output dir");
    }
    write_if_changed(&kt_path, kt_content).expect("failed to write AppLevelNames.kt");
}

/// 仅当内容变化时才写入，减少无意义的重新编译与 git 脏状态。
fn write_if_changed(path: &std::path::Path, content: String) -> std::io::Result<()> {
    if let Ok(existing) = std::fs::read_to_string(path) {
        if existing == content {
            return Ok(());
        }
    }
    std::fs::write(path, content)
}
