use std::mem;

use lsp_types::*;
use rayon::prelude::*;
use webgal_model::sentence::Scene;

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
    let content = scene.sentences()[position.line as usize].content;
    let character = offset_utf16_to_utf8(content, position.character);
    Position {
        character,
        ..position
    }
}

pub fn position_utf8_to_utf16(scene: &Scene, position: Position) -> Position {
    let content = scene.sentences()[position.line as usize].content;
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

pub fn text_edit_utf8_to_utf16(scene: &Scene, edit: TextEdit) -> TextEdit {
    TextEdit {
        range: range_utf8_to_utf16(scene, edit.range),
        ..edit
    }
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

pub fn formatting_utf8_to_utf16(scene: &Scene, edits: &mut [TextEdit]) {
    edits.par_iter_mut().for_each(|edit| {
        *edit = text_edit_utf8_to_utf16(scene, mem::take(edit));
    });
}
