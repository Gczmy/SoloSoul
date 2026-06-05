// SoloSoul Tauri Application Entrypoint

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    solosoul_app::run()
}
