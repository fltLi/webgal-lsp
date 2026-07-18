use std::ops;

use rayon::prelude::*;
use tower_lsp::lsp_types::*;
use webgal_language_core::sentence::*;

use crate::{
    project::Project,
    service::diagnose::{
        environment::diagnose_environment,
        syntax::{diagnose_format, diagnose_sentence_error},
    },
};

mod environment;
mod syntax;

/// 为项目提供诊断
///
/// # Behavior
/// * 存在 ERROR, WARNING, INFORMATION 三种级别, 对每个场景, 仅当不存在前两者时才推送 INFO 级别的诊断.
/// * 对于没有问题的场景, 会给出一个空诊断列表而不是过滤掉.
pub fn diagnose_project(project: &Project) -> Vec<(String, &Scene, Vec<Diagnostic>)> {
    project
        .resource()
        .scene
        .iter_recursively()
        .par_bridge()
        .filter_map(|(path, scene)| {
            let scene = scene.as_item()?;
            let diagnostics = diagnose_scene(scene, project);
            Some((path, scene, diagnostics))
        })
        .collect()
}

/// 为场景提供诊断
///
/// # Behavior
/// * 存在 ERROR, WARNING, INFORMATION 三种级别, 仅当不存在前两者时才推送 INFO 级别的诊断.
pub fn diagnose_scene(scene: &Scene, project: &Project) -> Vec<Diagnostic> {
    // 并行收集诊断
    let diagnostics: Vec<_> = scene
        .sentences()
        .par_iter()
        .enumerate()
        .filter_map(|(line, sentence)| {
            let mut diagnostics = Vec::new();
            diagnose_sentence(sentence, project, |diagnostic| diagnostics.push(diagnostic));
            (!diagnostics.is_empty()).then_some((line, diagnostics))
        })
        .collect();

    let has_error_or_warning = diagnostics
        .iter()
        .flat_map(|(_, diagnostics)| diagnostics)
        .any(|diagnostic| diagnostic.level != DiagnosticLevel::Information);

    // 正式推送诊断
    diagnostics
        .into_par_iter()
        .flat_map(|(line, diagnostics)| {
            diagnostics
                .into_iter()
                .filter_map(|diagnostic| {
                    // 含高于 info 级别诊断时, 过滤 info 级别诊断
                    let reserve =
                        !has_error_or_warning || diagnostic.level != DiagnosticLevel::Information;
                    reserve.then(|| diagnostic.into_diagnostic(line))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// 生成一条语句的诊断
fn diagnose_sentence<F>(sentence: &SentenceInfo, project: &Project, mut diagnose: F)
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
        project,
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
            // source: Some("webgal-language-server".to_string()),
            message,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
