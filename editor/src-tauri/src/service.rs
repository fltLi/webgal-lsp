pub mod browser;
pub mod fs;
pub mod game_config;
pub mod git;
pub mod highlight;
pub mod lsp;
pub mod preview;
pub mod snapshot;
pub mod startup;
// 视频导出目前只支持 Windows; 其他平台走 stub
#[cfg(all(feature = "video-export", windows))]
pub mod video_export;
#[cfg(not(all(feature = "video-export", windows)))]
#[path = "service/video_export_stub.rs"]
pub mod video_export;
pub mod voice;
