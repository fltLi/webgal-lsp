//! webgal-ink Tauri 后端入口。
//!
//! 提供三类能力:
//! * LSP 传输: [`service::lsp::start_server`] 启动内置 WebGAL LSP 的 WebSocket 服务。
//! * 实时预览: [`service::preview`] 启动本地静态服务器 + `/api/webgalsync` 网关。
//! * 配音工作流: [`service::voice`] 管理 GPT-SoVITS 进程、角色库与音频处理。
//!
//! 应用退出时主动关闭 LSP 服务、预览服务器与 GSOV 进程, 避免后台任务阻塞退出。

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::service::{lsp::LspState, preview::PreviewState, voice::VoiceState};

mod service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(LspState::default())
        .manage(Mutex::new(PreviewState::new()))
        .manage(Arc::new(VoiceState::default()))
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
            service::voice::voice_detect_runtime,
            service::voice::voice_default_launch_config,
            service::voice::voice_launch,
            service::voice::voice_shutdown,
            service::voice::voice_status,
            service::voice::voice_take_logs,
            service::voice::voice_probe,
            service::voice::voice_synthesize,
            service::voice::voice_set_model,
            service::voice::voice_parse_scene,
            service::voice::voice_apply_lines,
            service::voice::voice_apply_scene,
            service::voice::voice_probe_audio,
            service::voice::voice_trim_suggestion,
            service::voice::voice_waveform,
            service::voice::voice_normalize_reference,
            service::voice::voice_read_audio,
            service::voice::voice_list_characters,
            service::voice::voice_load_character,
            service::voice::voice_save_character,
            service::voice::voice_remove_character,
            service::voice::voice_create_character,
            service::voice::voice_add_reference,
            service::voice::voice_remove_reference,
            service::voice::voice_export_character,
            service::voice::voice_import_character,
            service::voice::voice_list_cache,
            service::voice::voice_store_cache,
            service::voice::voice_drop_cache,
            service::voice::voice_scan_references,
            service::voice::voice_realign,
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
            // 主动关闭 GSOV 进程 (优先请求服务端优雅退出)
            if let Some(voice) = app_handle.try_state::<Arc<VoiceState>>() {
                tauri::async_runtime::block_on(voice.shutdown());
            }
        }
    });
}
