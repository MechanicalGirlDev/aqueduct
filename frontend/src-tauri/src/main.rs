#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            aqueduct_server_tauri::setup_aqueduct(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            aqueduct_server_tauri::commands::aqueduct_dispatch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
