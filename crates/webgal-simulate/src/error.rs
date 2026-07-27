use derive_more::{Deref, Into};
use strum::Display;
use thiserror::Error;

// TODO: 实现 `feature = "lsp"` 下诊断到 lsp-types 的类型转换

/// 模拟执行诊断错误信息 (多场景)
#[derive(Debug, Clone, Default, Into, Deref)]
pub struct DiagnosticList(pub(crate) Vec<(String, Vec<Diagnostic>)>);

/// 模拟执行诊断错误信息
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
#[error("第 {} 行: {detail}", .line + 1)]
pub struct Diagnostic {
    pub line: usize,
    #[source]
    pub detail: DiagnosticKind,
}

impl Diagnostic {
    /// 诊断错误码
    pub fn code(&self) -> &'static str {
        self.detail.code()
    }

    /// 诊断错误级别
    pub fn level(&self) -> DiagnosticLevel {
        self.detail.level()
    }
}

/// 模拟执行诊断错误类型
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum DiagnosticKind {
    /// WG008
    #[error("语句依赖的{0} `{1}` 未定义")]
    UndefinedSymbol(SymbolKind, String),

    /// WG009
    #[error("内联表达式 `{0}` 执行错误: {1}")]
    ExpressionError(String, String),

    /// WG010
    #[error("条件表达式 `{0}` 结果为常量 `{1}`")]
    ConstantCondition(String, String),

    /// WG011
    ///
    /// # Notes
    /// 为避免出错中断造成的误报, 仅当无其他类型错误时才提供此诊断.
    #[error("语句从未执行")]
    Unused,

    /// WG012
    #[error("与上文重复的无意义执行效果: {0}")]
    RedundantEffect(String),

    /// WG013
    #[error("连续执行块以 wait 语句结尾, 被迫打断")]
    WaitAtEndOfChain,

    /// WG014
    #[error("模拟执行停止: {0}")]
    Stopped(#[from] StopReason),
}

/// 模拟执行停止的具体原因
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum StopReason {
    #[error("游戏正常结束")]
    NormalTermination,

    #[error("检查点移除重复状态或达到最大通过次数")]
    Checkpoint,

    #[error("用户输入语句缺少静态分析预设值或默认值")]
    MissingUserInputValue,

    #[error("涉及随机数生成，模拟执行不支持该特性")]
    RandomNumberInterrupt,
}

/// 符号种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum SymbolKind {
    #[strum(to_string = "变量")]
    Variable,
    #[strum(to_string = "标签")]
    Label,
    #[strum(to_string = "场景")]
    Scene,
    #[strum(to_string = "舞台对象")]
    StageObject,
}

impl DiagnosticKind {
    /// 诊断错误码
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndefinedSymbol(..) => "WG008",
            Self::ExpressionError(..) => "WG009",
            Self::ConstantCondition(..) => "WG010",
            Self::Unused => "WG011",
            Self::RedundantEffect(_) => "WG012",
            Self::WaitAtEndOfChain => "WG013",
            Self::Stopped(_) => "WG014",
        }
    }

    /// 诊断错误级别
    pub fn level(&self) -> DiagnosticLevel {
        match self {
            Self::UndefinedSymbol(..) => DiagnosticLevel::Error,
            Self::ExpressionError(..) => DiagnosticLevel::Error,
            Self::ConstantCondition(..) => DiagnosticLevel::Warning,
            Self::Unused => DiagnosticLevel::Warning,
            Self::RedundantEffect(_) => DiagnosticLevel::Warning,
            Self::WaitAtEndOfChain => DiagnosticLevel::Warning,
            Self::Stopped(_) => DiagnosticLevel::Hint,
        }
    }
}

/// 模拟执行诊断错误级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
    Hint,
}
