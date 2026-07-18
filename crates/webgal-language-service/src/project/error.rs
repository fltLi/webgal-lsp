use std::result;

use thiserror::Error;
use webgal_language_core::resource::ResourceKind;

/// 项目管理返回类型
pub type Result<T> = result::Result<T, Error>;

/// 项目管理错误信息
#[derive(Debug, Error)]
#[error("访问 {kind} 资源 `{path}` 出错: {detail}")]
pub struct Error {
    pub path: String,
    pub kind: ResourceKind,
    #[source]
    pub detail: ErrorKind,
}

/// 项目管理错误类型
#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error(transparent)]
    InvalidPath(#[from] PathError),

    #[error("获取文件内容失败: {0}")]
    Content(#[source] anyhow::Error),

    #[error("无法删除配置文件, 这会导致项目消失")]
    RemoveConfig,

    #[error(transparent)]
    Figure(#[from] FigureError),
}

/// 项目管理的路径错误类型
#[derive(Debug, Error)]
pub enum PathError {
    #[error("路径位于根目录之外")]
    OutsideRoot,

    #[error("路径段目录部分中存在文件")]
    CrashFile,

    #[error("路径指向目录")]
    IsFolder,

    #[error("路径不存在")]
    NotFound,
}

/// 项目管理的立绘错误类型
#[derive(Debug, Error)]
pub enum FigureError {
    #[error("WMDL 模型配置文件解析失败: {0}")]
    WmdlParse(#[source] serde_json::Error),

    #[error("依赖模型 `{0}` 导入失败")]
    ModelImport(String),
}
