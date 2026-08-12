use std::{collections::HashMap, fmt};

use lsp_types::*;
use once_cell::sync::Lazy;
use serde::Deserialize;
use webgal_language_core::sentence::{Scene, Sentence, SentenceInfo, SentenceLocation};

pub fn document_capability() -> HoverProviderCapability {
    HoverProviderCapability::Simple(true)
}

/// 悬浮文档
pub fn document(scene: &Scene, position: Position) -> Option<Hover> {
    // 定位输入
    let SentenceInfo {
        primary, sentence, ..
    } = scene.sentences().get(position.line as usize)?;

    // 查询文档
    let command = match sentence {
        Sentence::Say(_) => "say",
        Sentence::Comment(_) => "comment",
        _ => primary.command,
    };
    let documentation = match primary.locate(position.character as usize) {
        SentenceLocation::Command(_) => document_command(command),
        SentenceLocation::Content(_) => document_content(command),
        SentenceLocation::ArgumentName(index, _) => {
            document_argument(command, primary.arguments[index].0)
        }
        SentenceLocation::ArgumentValue(..) => None,
        SentenceLocation::Comment(_) => None,
        SentenceLocation::Other if matches!(sentence, Sentence::Comment(_)) => {
            document_comment_sentence()
        }
        SentenceLocation::Other => None,
    };
    documentation.map(|document| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: document.to_string(),
        }),
        range: None,
    })
}

static DOCUMENT: Lazy<HashMap<String, SentenceDocument>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../data/document.json"))
        .expect("WebGAL 离线文档 JSON 解析失败")
});

fn document_command(command: &str) -> Option<&'static Document> {
    DOCUMENT.get(command).map(|sentence| &sentence.command)
}

fn document_content(command: &str) -> Option<&'static Document> {
    DOCUMENT.get(command).map(|sentence| &sentence.content)
}

fn document_argument(command: &str, name: &str) -> Option<&'static Document> {
    DOCUMENT
        .get(command)
        .and_then(|sentence| sentence.arguments.get(name))
}

fn document_comment_sentence() -> Option<&'static Document> {
    DOCUMENT.get("comment").map(|sentence| &sentence.command)
}

#[derive(Deserialize)]
struct SentenceDocument {
    command: Document,
    content: Document,
    arguments: HashMap<String, Document>,
}

#[derive(Deserialize)]
struct Document {
    link: String,
    document: String,
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.document)?;
        write!(
            f,
            "\n\n---\nWebGAL 官方文档: [{}]({})",
            self.link, self.link
        )
    }
}
