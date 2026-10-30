mod commands;
mod kits;
mod model;
mod sbx;
mod settings;
mod terminal;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::detect_sbx,
            commands::list_sandboxes,
            commands::list_agents,
            commands::list_kits,
            commands::list_secrets,
            commands::list_mcp_servers,
            commands::read_kit_spec,
            commands::read_kit_host_env,
            commands::write_kit_host_env,
            commands::create_sandbox,
            commands::run_lifecycle,
            commands::detect_terminal,
            commands::open_sandbox_terminal,
            commands::ensure_workspace_dir,
            commands::path_exists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
