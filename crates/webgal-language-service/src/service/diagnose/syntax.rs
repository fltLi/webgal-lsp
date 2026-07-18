//! 单语句基础语法和独立语义检查

use webgal_language_core::sentence::{self, PrimarySentence, SentenceInfo};

use crate::service::diagnose::{DiagnosticLevel, PrimaryDiagnostic};

/// 将 [`webgal_language_core::sentence::Sentence`] 解析产生的错误转换为诊断信息
pub fn diagnose_sentence_error(
    primary: &PrimarySentence,
    error: &sentence::Error,
) -> Option<PrimaryDiagnostic> {
    use sentence::Error::*;

    let PrimarySentence {
        content, arguments, ..
    } = primary;

    match error {
        ContentType(error) => {
            let content = (*content)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(content),
                code: "WG002",
                level: DiagnosticLevel::Error,
                message: format!("语句主参数值 `{content}` 类型错误: {error}"),
            })
        }

        ArgumentType(index, error) => {
            let (name, value) = *arguments.get(*index)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(value.unwrap_or(name)),
                code: "WG002",
                level: DiagnosticLevel::Error,
                message: match value {
                    Some(value) => {
                        format!("语句参数 `{name}` 的值 `{value}` 类型错误: {error}")
                    }
                    None => format!("语句参数 `{name}` 的值类型错误: {error}"),
                },
            })
        }

        ArgumentRepeated(index) => {
            let (name, _) = *arguments.get(*index)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(name),
                code: "WG003",
                level: DiagnosticLevel::Warning,
                message: format!("语句参数 `{name}` 重复设置或与其他参数冲突"),
            })
        }

        ArgumentMissingDependencies(index, missings) => {
            let (name, _) = *arguments.get(*index)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(name),
                code: "WG004",
                level: DiagnosticLevel::Error,
                message: format!("语句中缺少参数 `{name}` 所依赖的相关参数: {missings:?}"),
            })
        }

        ArgumentObsolete(index, reason) => {
            let (name, _) = *arguments.get(*index)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(name),
                code: "WG005",
                level: DiagnosticLevel::Warning,
                message: format!("语句参数 `{name}` 已被弃用或不建议使用, 理由: {reason}"),
            })
        }

        ArgumentUnknown(index) => {
            let (name, _) = *arguments.get(*index)?;
            Some(PrimaryDiagnostic {
                span: primary.get_span(name),
                code: "WG006",
                level: DiagnosticLevel::Warning,
                message: format!("语句参数 `{name}` 未知或无法识别"),
            })
        }
    }
}

/// 检查语句格式化
pub fn diagnose_format(sentence: &SentenceInfo) -> Option<PrimaryDiagnostic> {
    let expected = sentence.to_string();
    expected.ne(sentence.content).then(|| PrimaryDiagnostic {
        span: 0..sentence.content.len(),
        code: "WG001",
        level: DiagnosticLevel::Information,
        message: format!("语句格式不规范，应为：`{expected}`"),
    })
}
