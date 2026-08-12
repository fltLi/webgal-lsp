use lsp_types::*;
use webgal_language_core::sentence::{Scene, SentenceInfo, SentenceLocation};

use crate::{
    project::Project,
    service::complete::{argument::Complete, command::complete_command, comment::complete_comment},
};

mod argument;
mod command;
mod comment;

pub fn complete_capability() -> CompletionOptions {
    CompletionOptions {
        trigger_characters: Some(vec![
            ":".to_string(),  // 主参数
            "-".to_string(),  // 参数名
            "=".to_string(),  // 参数值
            "/".to_string(),  // 路径 / 立绘动作表情
            "\\".to_string(), // 路径 / 立绘动作表情
            "\"".to_string(), // JSON
        ]),
        completion_item: Some(CompletionOptionsCompletionItem {
            label_details_support: Some(true),
        }),
        ..Default::default()
    }
}

/// 语句补全
pub fn complete(scene: &Scene, position: Position, project: &Project) -> Vec<CompletionItem> {
    // 定位输入
    let SentenceInfo {
        primary, sentence, ..
    } = match scene.sentences().get(position.line as usize) {
        Some(sentence) => sentence,
        None => return Vec::default(),
    };

    // 转发补全
    let completions = match primary.locate(position.character as usize) {
        SentenceLocation::Command(input) => complete_command(input, position, project),
        SentenceLocation::Content(input) => sentence.complete_content(input, position, project),
        SentenceLocation::ArgumentName(_, input) => {
            sentence.complete_argument_name(input, position, project)
        }
        SentenceLocation::ArgumentValue(_, name, input) => {
            sentence.complete_argument_value(name, input, position, project)
        }
        SentenceLocation::Comment(input) => complete_comment(input, position),
        SentenceLocation::Other => Vec::default(),
    };
    completions.into_iter().map(From::from).collect()
}

struct PrimaryCompletion {
    // 描述
    name: String,
    kind: CompletionItemKind,
    description: Option<String>,
    document: Option<MarkupContent>,
    sort_key: Option<String>,
    // 执行
    span: Range,
    insert_text: Option<String>,
}

impl From<PrimaryCompletion> for CompletionItem {
    fn from(value: PrimaryCompletion) -> Self {
        let PrimaryCompletion {
            name,
            kind,
            description,
            document,
            sort_key,
            span,
            insert_text,
        } = value;

        let insert_text = insert_text.unwrap_or_else(|| name.clone());
        let insert_text_format = if insert_text.ends_with("$0") {
            InsertTextFormat::SNIPPET
        } else {
            InsertTextFormat::PLAIN_TEXT
        };

        Self {
            label: name,
            label_details: Some(CompletionItemLabelDetails {
                detail: None,
                description,
            }),
            kind: Some(kind),
            documentation: document.map(Documentation::MarkupContent),
            sort_text: sort_key,
            insert_text_format: Some(insert_text_format),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: span,
                new_text: insert_text,
            })),
            ..Default::default()
        }
    }
}

// 兼容 json-language-service 返回的 CompletionItem, 请不要在其他地方执行!
impl From<CompletionItem> for PrimaryCompletion {
    fn from(value: CompletionItem) -> Self {
        let CompletionItem {
            label,
            label_details,
            kind,
            sort_text,
            text_edit,
            ..
        } = value;

        let description = label_details.and_then(|ld| ld.description);
        let (span, insert_text) = match text_edit {
            Some(CompletionTextEdit::Edit(edit)) => (edit.range, Some(edit.new_text)),
            Some(CompletionTextEdit::InsertAndReplace(insert_and_replace)) => {
                (insert_and_replace.insert, Some(insert_and_replace.new_text))
            }
            None => (Range::default(), None),
        };

        Self {
            name: label,
            kind: kind.unwrap_or(CompletionItemKind::TEXT),
            description,
            document: None,
            sort_key: sort_text,
            span,
            insert_text,
        }
    }
}

fn make_span(end: Position, len: usize) -> Range {
    let Position { line, character } = end;
    let start = Position {
        line,
        character: character.saturating_sub(len as u32),
    };
    Range { start, end }
}
