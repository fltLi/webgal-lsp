//! WebGAL 项目快照打包
//!
//! 将开发中的 WebGAL 项目打包为压缩包, 用于分发或备份:
//! * 游戏资源 (如背景, 立绘, 音乐) 仅打包被语句与配置引用的部分.
//! * 其余资源全部保留.

pub use config::*;
pub use error::*;
pub use pack::*;

mod config;
mod error;
mod pack;
