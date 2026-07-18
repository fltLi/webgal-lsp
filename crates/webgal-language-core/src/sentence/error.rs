use std::result;

#[cfg(feature = "serde")]
use serde_with::SerializeDisplay;
use thiserror::Error;

/// 语句解析返回类型
pub type Result<T> = result::Result<T, Error>;

/// 语句解析错误类型
#[derive(Debug, Error)]
#[cfg_attr(feature = "serde", derive(SerializeDisplay))]
pub enum Error {
    #[error("语句主参数值类型错误: {0}")]
    ContentType(#[source] anyhow::Error),

    #[error("语句第 {} 条参数值类型错误: {}", .0 + 1, .1)]
    ArgumentType(usize, #[source] anyhow::Error),

    #[error("语句第 {} 条参数重复定义", .0 + 1)]
    ArgumentRepeated(usize),

    #[error("语句第 {} 条参数缺少所需的依赖参数: {}", .0 + 1, .1.join(", "))]
    ArgumentMissingDependencies(usize, Vec<&'static str>),

    #[error("语句第 {} 条参数已弃用, 理由: {}", .0 + 1, .1)]
    ArgumentObsolete(usize, &'static str),

    #[error("语句第 {} 条参数未知", .0 + 1)]
    ArgumentUnknown(usize),
}
