use lsp_types::*;
use rayon::prelude::*;
use webgal_language_core::sentence::Scene;

pub fn format_capability() -> OneOf<bool, DocumentFormattingOptions> {
    OneOf::Left(true)
}

/// 场景格式化
pub fn format(scene: &Scene) -> Vec<TextEdit> {
    scene
        .sentences()
        .par_iter()
        .enumerate()
        .filter(|(_, sentence)| !sentence.should_skip_formatting())
        .map(|(line, sentence)| TextEdit {
            range: Range {
                start: Position {
                    line: line as u32,
                    character: 0,
                },
                end: Position {
                    line: line as u32,
                    character: sentence.content.len() as u32,
                },
            },
            new_text: sentence.to_string(),
        })
        .collect()
}
