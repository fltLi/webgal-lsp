use derive_more::{Deref, Into, IntoIterator};
#[cfg(feature = "lsp")]
use lsp_types::{Diagnostic as LspDiagnostic, *};
use strum::Display;
use thiserror::Error;
#[cfg(feature = "lsp")]
use webgal_language_core::sentence::PrimarySentence;

/// 模拟执行诊断错误信息 (多场景)
#[derive(Debug, Clone, Default, Into, IntoIterator, Deref)]
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

    /// 转换为 LSP 诊断信息
    ///
    /// # Arguments
    /// * `f` - 传入行号, 获得原始语句借用.
    #[cfg(feature = "lsp")]
    pub fn to_lsp_diagnostic<'a, 'b: 'a, F>(&self, f: F) -> LspDiagnostic
    where
        F: FnOnce(usize) -> &'a PrimarySentence<'b>,
    {
        let span = Range {
            start: Position {
                line: self.line as u32,
                character: 0,
            },
            end: Position {
                line: self.line as u32,
                character: f(self.line).len() as u32,
            },
        };

        LspDiagnostic {
            range: span,
            severity: Some(self.level().into()),
            code: Some(NumberOrString::String(self.code().to_string())),
            message: self.detail.to_string(),
            ..Default::default()
        }
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
    #[error("覆盖当前连续执行块中已有效果: {0}")]
    OverriddenEffect(String),

    /// WG014
    #[error("连续执行块以 wait 语句结尾, 被迫打断")]
    WaitAtEndOfChain,

    /// WG015
    #[error("模拟执行停止: {0}")]
    Stopped(#[from] StopReason),

    /// WG016
    #[error("对话内容过长: 预估行数 {0} 行, 超过最大 3 行")]
    DialogueTooLong(usize),
}

impl DiagnosticKind {
    pub(crate) fn prevents_unused_check(&self) -> bool {
        matches!(self, Self::Stopped(reason) if reason.prevents_unused_check())
    }
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

impl StopReason {
    pub(crate) fn prevents_unused_check(&self) -> bool {
        !matches!(self, StopReason::NormalTermination)
    }
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
    #[strum(to_string = "背景")]
    Background,
    #[strum(to_string = "立绘")]
    Figure,
    #[strum(to_string = "效果音")]
    Sound,
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
            Self::OverriddenEffect(_) => "WG013",
            Self::WaitAtEndOfChain => "WG014",
            Self::Stopped(_) => "WG015",
            Self::DialogueTooLong(_) => "WG016",
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
            Self::OverriddenEffect(_) => DiagnosticLevel::Warning,
            Self::WaitAtEndOfChain => DiagnosticLevel::Warning,
            Self::Stopped(_) => DiagnosticLevel::Hint,
            Self::DialogueTooLong(_) => DiagnosticLevel::Warning,
        }
    }
}

/// 模拟执行诊断错误级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticLevel {
    Hint,
    Info,
    Warning,
    Error,
}

#[cfg(feature = "lsp")]
impl From<DiagnosticLevel> for DiagnosticSeverity {
    fn from(value: DiagnosticLevel) -> Self {
        match value {
            DiagnosticLevel::Error => Self::ERROR,
            DiagnosticLevel::Warning => Self::WARNING,
            DiagnosticLevel::Info => Self::INFORMATION,
            DiagnosticLevel::Hint => Self::HINT,
        }
    }
}
