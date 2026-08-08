//! LSP 类型转换

use lsp_types::*;

use crate::{Completion, IdentKind, Schema};

impl Schema {
    /// 宽松解析 JSON 字符串并提供补全 (LSP)
    pub fn complete_lsp(&self, s: &str, position: Position) -> Vec<CompletionItem> {
        self.complete(s)
            .into_iter()
            .map(|completion| completion.to_lsp_completion(position))
            .collect()
    }
}

impl IdentKind {
    pub fn to_lsp_completion_kind(&self) -> CompletionItemKind {
        match self {
            Self::Key => CompletionItemKind::PROPERTY,
            Self::String => CompletionItemKind::TEXT,
            Self::Number | Self::Bool => CompletionItemKind::VALUE,
        }
    }
}

impl From<IdentKind> for CompletionItemKind {
    fn from(value: IdentKind) -> Self {
        value.to_lsp_completion_kind()
    }
}

impl Completion {
    pub fn to_lsp_completion(&self, position: Position) -> CompletionItem {
        let Self {
            name,
            kind,
            len,
            description,
        } = self;

        let Position { line, character } = position;
        let start = Position {
            line,
            character: character.saturating_sub(*len as u32),
        };
        let span = Range {
            start,
            end: position,
        };

        CompletionItem {
            label: name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: None,
                description: Some(description.clone()),
            }),
            kind: Some(kind.to_lsp_completion_kind()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: span,
                new_text: name.clone(),
            })),
            ..Default::default()
        }
    }
}
