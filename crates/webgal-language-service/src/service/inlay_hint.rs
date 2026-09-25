use lsp_types::*;
use webgal_language_core::sentence::{Scene, Sentence, is_implicit_vocal_argument};

use crate::{project::Project, service::position_in_range};

pub fn inlay_hint_capability() -> OneOf<bool, InlayHintServerCapabilities> {
    OneOf::Left(true)
}

/// 内联提示
///
/// # Behavior
/// * 为变量定义附加类型推断, 包括 `setVar` / `callScene: -var=expr -writeReturnTo=var`.
/// * 为对话语句的配音参数语法糖附加 `-vocal=` 参数名.
pub fn inlay_hint(scene_path: &str, span: Range, project: &Project) -> Option<Vec<InlayHint>> {
    let scene = project.resource().scene.get(scene_path)?.as_item()?;
    let variable_hints = inlay_hint_variable_definitions(scene_path, span, project);
    let vocal_hints = inlay_hint_vocal_arguments(scene, span);
    Some(variable_hints.chain(vocal_hints).collect())
}

fn inlay_hint_variable_definitions(
    scene_path: &str,
    span: Range,
    project: &Project,
) -> impl Iterator<Item = InlayHint> {
    project.variable().values().flat_map(move |variable| {
        variable.definitions.iter().filter_map(move |definition| {
            let kind = definition.kind?;
            let position = Position {
                line: definition.location.line as u32,
                character: definition.location.span.end as u32,
            };
            (definition.location.scene == scene_path && position_in_range(position, span)).then(
                || InlayHint {
                    position,
                    label: InlayHintLabel::String(format!(": {kind}")),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: Some(true),
                    data: None,
                },
            )
        })
    })
}

fn inlay_hint_vocal_arguments(scene: &Scene, span: Range) -> impl Iterator<Item = InlayHint> {
    scene
        .sentences()
        .iter()
        .enumerate()
        .filter(|(_, sentence)| matches!(&sentence.sentence, Sentence::Say(s) if s.vocal.is_some()))
        .filter_map(move |(line, sentence)| {
            let &(vocal, _) = sentence
                .primary
                .arguments
                .iter()
                .find(|(name, value)| value.is_none() && is_implicit_vocal_argument(name))?;
            let position = Position {
                line: line as u32,
                character: sentence.primary.get_span(vocal).start as u32,
            };
            position_in_range(position, span).then(|| InlayHint {
                position,
                label: InlayHintLabel::String("vocal=".to_string()),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: Some(vec![TextEdit {
                    range: Range {
                        start: position,
                        end: position,
                    },
                    new_text: "vocal=".to_string(),
                }]),
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            })
        })
}
