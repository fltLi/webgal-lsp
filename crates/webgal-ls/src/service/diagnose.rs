use std::ops;

use tower_lsp::lsp_types::*;
use webgal_model::sentence::*;

use crate::{
    context::Context,
    service::diagnose::{
        environment::diagnose_environment,
        syntax::{diagnose_format, diagnose_sentence_error},
    },
};

mod environment;
mod syntax;

/// 为场景提供诊断
///
/// # Behavior
/// * 存在 ERROR, WARNING, INFORMATION 三种级别, 仅当不存在前两者时才推送 INFO 级别的诊断.
pub fn diagnose(scene: &Scene, context: &Context) -> Vec<Diagnostic> {
    let mut filter_info = false;
    let mut diagnostics = Vec::new();

    for (line, sentence) in scene.sentences().iter().enumerate() {
        diagnose_sentence(sentence, context, |diagnostic| match diagnostic.level {
            DiagnosticLevel::Information => {
                if !filter_info {
                    diagnostics.push(diagnostic.into_diagnostic(line));
                }
            }
            DiagnosticLevel::Error | DiagnosticLevel::Warning => {
                if !filter_info {
                    filter_info = true;
                    diagnostics.clear();
                }
                diagnostics.push(diagnostic.into_diagnostic(line));
            }
        });
    }

    diagnostics
}

/// 生成一条语句的诊断
fn diagnose_sentence<F>(sentence: &SentenceInfo, context: &Context, mut diagnose: F)
where
    F: FnMut(PrimaryDiagnostic),
{
    // 包装提交函数, 过滤 nolints
    let mut diagnose = |diagnostic: PrimaryDiagnostic| {
        if !sentence.contains_nolint(diagnostic.code) {
            diagnose(diagnostic);
        }
    };

    // 语法检查
    for error in &sentence.errors {
        if let Some(diagnostic) = diagnose_sentence_error(&sentence.primary, error) {
            diagnose(diagnostic);
        }
    }
    if let Some(diagnostic) = diagnose_format(sentence) {
        diagnose(diagnostic);
    }

    // 环境诊断
    diagnose_environment(
        sentence.content,
        &sentence.primary,
        &sentence.sentence,
        context,
        &mut diagnose,
    );
}

struct PrimaryDiagnostic {
    span: ops::Range<usize>,
    code: &'static str,
    level: DiagnosticLevel,
    message: String,
}

impl PrimaryDiagnostic {
    fn into_diagnostic(self, line: usize) -> Diagnostic {
        let Self {
            span: ops::Range { start, end },
            code,
            level,
            message,
        } = self;

        Diagnostic {
            range: Range {
                start: Position {
                    line: line as u32,
                    character: start as u32,
                },
                end: Position {
                    line: line as u32,
                    character: end as u32,
                },
            },
            severity: Some(level.into()),
            code: Some(NumberOrString::String(code.to_string())),
            source: Some("webgal-ls".to_string()),
            message,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy)]
enum DiagnosticLevel {
    Information,
    Warning,
    Error,
}

impl From<DiagnosticLevel> for DiagnosticSeverity {
    fn from(value: DiagnosticLevel) -> Self {
        match value {
            DiagnosticLevel::Error => Self::ERROR,
            DiagnosticLevel::Warning => Self::WARNING,
            DiagnosticLevel::Information => Self::INFORMATION,
        }
    }
}
