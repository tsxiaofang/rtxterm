mod crypt;
mod download;
mod proxy;
mod server;
mod ssh;
mod upload;

use download::ssh_download;
use server::{
    ssh_add_server, ssh_config_all, ssh_del_server, ssh_get_servers, ssh_login, ssh_server_detail,
    ssh_set_config, ssh_update_server, ServerContext, ServerMgr,
};
use ssh::{ssh_close, ssh_connect, ssh_send, SShMgr};
use tauri::Manager;
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};
use upload::ssh_upload;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(w) = app.get_webview_window("main") {
                w.restore_state(StateFlags::all()).ok();
                if !w.is_visible().unwrap_or_default() {
                    w.show().ok();
                }
            }
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            ssh_connect,
            ssh_send,
            ssh_close,
            ssh_login,
            ssh_get_servers,
            ssh_add_server,
            ssh_del_server,
            ssh_server_detail,
            ssh_update_server,
            ssh_upload,
            ssh_download,
            ssh_config_all,
            ssh_set_config,
        ])
        .on_window_event(|w, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let app = w.app_handle();
                app.save_window_state(StateFlags::all()).ok();
            }
        })
        .manage(SShMgr::default())
        .manage(ServerContext::new(ServerMgr::new()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
