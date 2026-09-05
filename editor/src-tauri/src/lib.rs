//! webgal-ink Tauri 后端入口。
//!
//! 提供两类能力:
//! * LSP 传输: [`service::lsp::start_server`] 启动内置 WebGAL LSP 的 WebSocket 服务。
//! * 实时预览: [`service::preview`] 启动本地静态服务器 + `/api/webgalsync` 网关。
//!
//! 应用退出时主动关闭 LSP 服务与预览服务器, 避免后台任务阻塞退出。

use tauri::Manager;
use tokio::sync::Mutex;

use crate::service::{lsp::LspState, preview::PreviewState};

mod service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(LspState::default())
        .manage(Mutex::new(PreviewState::new()))
        .invoke_handler(tauri::generate_handler![
            service::fs::copy_directory,
            service::fs::move_to_trash,
            service::git::git_status,
            service::git::git_init,
            service::git::git_set_identity,
            service::git::git_stage,
            service::git::git_stage_all,
            service::git::git_unstage,
            service::git::git_unstage_all,
            service::git::git_discard,
            service::git::git_discard_all,
            service::git::git_commit,
            service::git::git_log,
            service::git::git_commit_files,
            service::git::git_diff,
            service::git::git_commit_diff,
            service::git::git_restore,
            service::git::git_restore_all,
            service::git::git_file_changes,
            service::git::git_file_region,
            service::lsp::start_server,
            service::highlight::semantic_token_types,
            service::highlight::highlight_scene,
            service::preview::start_preview_server,
            service::preview::add_static_site,
            service::preview::set_active_preview_session,
            service::preview::set_embedded_preview_launch_id,
            service::preview::send_preview_command,
            service::snapshot::pack_snapshot,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            // 主动关闭 LSP 服务 (终止后台 accept 循环)
            if let Some(lsp) = app_handle.try_state::<LspState>() {
                if let Some(handle) = tauri::async_runtime::block_on(lsp.server_task.lock()).take()
                {
                    handle.abort();
                }
            }
            // 主动关闭预览服务器
            if let Some(preview) = app_handle.try_state::<Mutex<PreviewState>>() {
                let mut guard = tauri::async_runtime::block_on(preview.lock());
                if let Some(handle) = guard.server_handle.take() {
                    let _ = handle.shutdown_tx.send(());
                }
            }
        }
    });
}
