// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

mod register_handlers;
use register_handlers::data_center::performance_evaluation::register_case_data_handler;

mod states;
use states::data_center::performance_evaluation::case_data::database::PerformanceEvaluationCaseDataState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handler = app.handle();
            register_case_data_handler(handler);
            let db = PerformanceEvaluationCaseDataState::new(handler);
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
