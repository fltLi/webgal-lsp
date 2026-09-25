use std::{borrow::Cow, mem};

use lsp_types::*;
use path_tree::canonicalize;
use percent_encoding::percent_decode_str;
use rayon::prelude::*;
use webgal_language_core::sentence::Scene;

use crate::project::Project;

pub fn offset_utf16_to_utf8(content: &str, offset: u32) -> u32 {
    let mut utf8_pos: usize = 0;
    let mut utf16_pos: usize = 0;
    for ch in content.chars() {
        let ch_utf16 = ch.len_utf16();
        let ch_utf8 = ch.len_utf8();
        if utf16_pos + ch_utf16 > offset as usize {
            break;
        }
        utf16_pos += ch_utf16;
        utf8_pos += ch_utf8;
    }
    utf8_pos as u32
}

pub fn offset_utf8_to_utf16(content: &str, offset: u32) -> u32 {
    let offset = offset as usize;
    let mut utf8_pos: usize = 0;
    let mut utf16_pos: usize = 0;
    for ch in content.chars() {
        let next_utf8 = utf8_pos + ch.len_utf8();
        if next_utf8 > offset {
            break;
        }
        utf8_pos = next_utf8;
        utf16_pos += ch.len_utf16();
    }
    utf16_pos as u32
}

pub fn position_utf16_to_utf8(scene: &Scene, position: Position) -> Position {
    let Some(content) = scene
        .sentences()
        .get(position.line as usize)
        .map(|sentence| sentence.content)
    else {
        return position;
    };
    let character = offset_utf16_to_utf8(content, position.character);
    Position {
        character,
        ..position
    }
}

pub fn position_utf8_to_utf16(scene: &Scene, position: Position) -> Position {
    let Some(content) = scene
        .sentences()
        .get(position.line as usize)
        .map(|sentence| sentence.content)
    else {
        return position;
    };
    let character = offset_utf8_to_utf16(content, position.character);
    Position {
        character,
        ..position
    }
}

pub fn range_utf16_to_utf8(scene: &Scene, range: Range) -> Range {
    Range {
        start: position_utf16_to_utf8(scene, range.start),
        end: position_utf16_to_utf8(scene, range.end),
    }
}

pub fn range_utf8_to_utf16(scene: &Scene, range: Range) -> Range {
    Range {
        start: position_utf8_to_utf16(scene, range.start),
        end: position_utf8_to_utf16(scene, range.end),
    }
}

pub fn location_utf8_to_utf16(
    project: &Project,
    project_root: &str,
    location: Location,
) -> Option<Location> {
    let absolute_path = location.uri.to_string();
    let absolute_path = percent_decode_str(&absolute_path)
        .decode_utf8()
        .unwrap_or(Cow::Borrowed(&absolute_path));
    let relative_path = canonicalize(
        absolute_path
            .strip_prefix(project_root)?
            .trim_start_matches('/')
            .strip_prefix("scene/")?,
    )?;
    let scene = project.resource().scene.get(&relative_path)?.as_item()?;
    Some(Location {
        range: range_utf8_to_utf16(scene, location.range),
        ..location
    })
}

pub fn locations_utf8_to_utf16(project: &Project, project_root: &str, locations: &mut [Location]) {
    locations.par_iter_mut().for_each(|location| {
        *location = location_utf8_to_utf16(project, project_root, location.clone())
            .expect("Location 指向的位置无效");
    });
}

pub fn text_edit_utf8_to_utf16(scene: &Scene, edit: TextEdit) -> TextEdit {
    TextEdit {
        range: range_utf8_to_utf16(scene, edit.range),
        ..edit
    }
}

pub fn text_edits_utf8_to_utf16(scene: &Scene, edits: &mut [TextEdit]) {
    edits.par_iter_mut().for_each(|edit| {
        *edit = text_edit_utf8_to_utf16(scene, mem::take(edit));
    });
}

// -------- service --------

pub fn diagnostics_utf8_to_utf16(scene: &Scene, diagnostics: &mut [Diagnostic]) {
    diagnostics.par_iter_mut().for_each(|diagnostic| {
        *diagnostic = diagnostic_utf8_to_utf16(scene, mem::take(diagnostic));
    });
}

pub fn diagnostic_utf8_to_utf16(scene: &Scene, diagnostic: Diagnostic) -> Diagnostic {
    Diagnostic {
        range: range_utf8_to_utf16(scene, diagnostic.range),
        ..diagnostic
    }
}

pub fn document_utf8_to_utf16(scene: &Scene, document: &mut Hover) {
    if let Some(ref mut range) = document.range {
        *range = range_utf8_to_utf16(scene, *range);
    }
}

pub fn highlights_utf8_to_utf16(scene: &Scene, tokens: &mut [SemanticToken]) {
    let mut current_line = 0;
    let mut current_byte_pos = 0; // 当前行内的字节偏移
    let mut prev_utf16_start = 0; // 上一个令牌的绝对 UTF-16 起始 (用于计算同行相对偏移)

    for token in tokens.iter_mut() {
        let delta_line = token.delta_line as usize;
        let delta_start = token.delta_start as usize;
        let length = token.length as usize;

        // 计算当前令牌的绝对字节起始位置 (相对于行首)
        if delta_line > 0 {
            // 跨行: current_byte_pos 直接设为 delta_start (相对新行行首)
            current_line += delta_line;
            current_byte_pos = delta_start;
        } else {
            // 同行: 累加 delta_start
            current_byte_pos += delta_start;
        }

        // 获取当前行内容
        let line_content = scene.sentences()[current_line].content;

        // 计算 UTF-16 偏移
        let utf16_start = offset_utf8_to_utf16(line_content, current_byte_pos as u32) as usize;
        let utf16_end =
            offset_utf8_to_utf16(line_content, (current_byte_pos + length) as u32) as usize;
        let utf16_len = utf16_end - utf16_start;

        // 重新计算 delta_start (保持相对偏移语义)
        let new_delta_start = if delta_line > 0 {
            utf16_start // 跨行时, 相对新行行首
        } else {
            utf16_start - prev_utf16_start // 同行时, 相对上一个令牌的起始
        };

        // 更新令牌
        token.delta_start = new_delta_start as u32;
        token.length = utf16_len as u32;

        // 更新状态
        prev_utf16_start = utf16_start;
        current_byte_pos += length; // 移动到当前令牌结束位置
    }
}

pub fn inlay_hints_utf8_to_utf16(scene: &Scene, hints: &mut [InlayHint]) {
    hints.par_iter_mut().for_each(|hint| {
        *hint = inlay_hint_utf8_to_utf16(scene, hint.clone());
    });
}

pub fn inlay_hint_utf8_to_utf16(scene: &Scene, hint: InlayHint) -> InlayHint {
    InlayHint {
        position: position_utf8_to_utf16(scene, hint.position),
        text_edits: hint.text_edits.map(|edits| {
            edits
                .into_iter()
                .map(|edit| text_edit_utf8_to_utf16(scene, edit))
                .collect()
        }),
        ..hint
    }
}

pub fn completions_utf8_to_utf16(scene: &Scene, completions: &mut [CompletionItem]) {
    completions.par_iter_mut().for_each(|completion| {
        *completion = completion_utf8_to_utf16(scene, mem::take(completion));
    });
}

pub fn completion_utf8_to_utf16(scene: &Scene, completion: CompletionItem) -> CompletionItem {
    CompletionItem {
        text_edit: completion.text_edit.map(|edit| match edit {
            CompletionTextEdit::Edit(edit) => {
                CompletionTextEdit::Edit(text_edit_utf8_to_utf16(scene, edit))
            }
            _ => unimplemented!(),
        }),
        ..completion
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    #[test]
    fn offset_utf16_to_utf8_ascii() {
        let s = "hello";
        assert_eq!(offset_utf16_to_utf8(s, 0), 0);
        assert_eq!(offset_utf16_to_utf8(s, 1), 1);
        assert_eq!(offset_utf16_to_utf8(s, 5), 5);
        assert_eq!(offset_utf16_to_utf8(s, 6), 5); // 超出最大偏移, 返回总 UTF-8 长度
    }

    #[test]
    fn offset_utf8_to_utf16_ascii() {
        let s = "hello";
        assert_eq!(offset_utf8_to_utf16(s, 0), 0);
        assert_eq!(offset_utf8_to_utf16(s, 1), 1);
        assert_eq!(offset_utf8_to_utf16(s, 5), 5);
        assert_eq!(offset_utf8_to_utf16(s, 6), 5);
    }

    #[test]
    fn offset_utf16_to_utf8_chinese() {
        let s = "你好世界";
        // 每个中文字符 UTF-16 长度 1, UTF-8 长度 3
        assert_eq!(offset_utf16_to_utf8(s, 0), 0);
        assert_eq!(offset_utf16_to_utf8(s, 1), 3);
        assert_eq!(offset_utf16_to_utf8(s, 2), 6);
        assert_eq!(offset_utf16_to_utf8(s, 3), 9);
        assert_eq!(offset_utf16_to_utf8(s, 4), 12); // 总 UTF-8 长度
    }

    #[test]
    fn offset_utf8_to_utf16_chinese() {
        let s = "你好世界";
        assert_eq!(offset_utf8_to_utf16(s, 0), 0);
        assert_eq!(offset_utf8_to_utf16(s, 3), 1);
        assert_eq!(offset_utf8_to_utf16(s, 6), 2);
        assert_eq!(offset_utf8_to_utf16(s, 9), 3);
        assert_eq!(offset_utf8_to_utf16(s, 12), 4); // 总 UTF-16 长度
    }

    #[test]
    fn offset_utf16_to_utf8_mixed() {
        let s = "a😀b";
        assert_eq!(offset_utf16_to_utf8(s, 0), 0);
        assert_eq!(offset_utf16_to_utf8(s, 1), 1); // 'a' 结束
        assert_eq!(offset_utf16_to_utf8(s, 2), 1); // 低代理项, 映射到字符起始
        assert_eq!(offset_utf16_to_utf8(s, 3), 5); // emoji 结束
        assert_eq!(offset_utf16_to_utf8(s, 4), 6); // 超出
    }

    #[test]
    fn offset_utf8_to_utf16_mixed() {
        let s = "a😀b";
        assert_eq!(offset_utf8_to_utf16(s, 0), 0);
        assert_eq!(offset_utf8_to_utf16(s, 1), 1); // 'a' 结束
        assert_eq!(offset_utf8_to_utf16(s, 5), 3); // emoji 结束 (起始位置 1 + 4 = 5)
        assert_eq!(offset_utf8_to_utf16(s, 6), 4); // 全部结束
        assert_eq!(offset_utf8_to_utf16(s, 7), 4); // 超出
    }

    #[test]
    fn position_conversion_accepts_eof_line() {
        let scene = Scene::from("hello\n");
        let position = Position {
            line: 1,
            character: 0,
        };
        assert_eq!(position_utf16_to_utf8(&scene, position), position);
        assert_eq!(position_utf8_to_utf16(&scene, position), position);
    }

    #[test]
    fn locations_conversion_uses_target_scene() {
        let mut project = Project::new(webgal_language_core::resource::Config::default());
        project
            .insert("scene/source.txt", || Ok("setVar:value=1;".to_string()))
            .unwrap();
        project
            .insert("scene/target.txt", || Ok("say:你好x;".to_string()))
            .unwrap();

        let mut locations = vec![Location {
            uri: "file:///project/scene/target.txt".parse().unwrap(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 10,
                },
                end: Position {
                    line: 0,
                    character: 11,
                },
            },
        }];

        locations_utf8_to_utf16(&project, "file:///project", &mut locations);

        assert_eq!(
            locations[0].range,
            Range {
                start: Position {
                    line: 0,
                    character: 6,
                },
                end: Position {
                    line: 0,
                    character: 7,
                },
            }
        );
    }
}
