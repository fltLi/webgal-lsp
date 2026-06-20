use std::ops;

use tower_lsp::lsp_types::*;
use webgal_model::sentence::{Scene, SentenceInfo};

// TODO: 精细化高亮

pub fn highlight_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
        SemanticTokensRegistrationOptions {
            text_document_registration_options: TextDocumentRegistrationOptions {
                document_selector: Some(vec![DocumentFilter {
                    language: Some("webgal".to_string()),
                    scheme: Some("file".to_string()),
                    pattern: Some("**/scene/**/*.txt".to_string()),
                }]),
            },
            semantic_tokens_options: SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: TokenType::all().to_vec(),
                    token_modifiers: vec![],
                },
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
            static_registration_options: StaticRegistrationOptions::default(),
        },
    )
}

/// 为场景提供语义高亮
pub fn highlight(scene: &Scene) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut last_line = 0;

    for (line, sentence) in scene.sentences().iter().enumerate() {
        let mut last_end = 0;
        tokens.extend(highlight_sentence(sentence).into_iter().map(
            |PrimaryToken { start, end, kind }| {
                let delta_line = if line == last_line {
                    0
                } else {
                    let delta = (line - last_line) as u32;
                    last_line = line;
                    delta
                };

                let delta_start = (start - last_end) as u32;
                let length = (end - start) as u32;
                last_end = end;

                SemanticToken {
                    delta_line,
                    delta_start,
                    length,
                    token_type: kind.to_id(),
                    token_modifiers_bitset: 0,
                }
            },
        ));
    }

    tokens
}

/// 生成一条语句的高亮
fn highlight_sentence(sentence: &SentenceInfo) -> Vec<PrimaryToken> {
    let SentenceInfo {
        content,
        primary,
        sentence,
        ..
    } = sentence;

    let mut tokens = Vec::new();
    let mut push_token = |text, kind| {
        let ops::Range { start, end } = primary.get_span(text);
        tokens.push(PrimaryToken { start, end, kind });
    };

    // 语句类型高亮
    push_token(
        primary.command,
        if sentence.is_say() {
            TokenType::Variable
        } else {
            TokenType::Function
        },
    );

    // 主参数高亮
    if let Some(content) = primary.content
        && sentence.is_say()
    {
        push_token(content, TokenType::String);
    }

    // 参数高亮
    for (name, _) in primary.arguments.iter() {
        push_token(name, TokenType::Property);
    }

    // 注释高亮
    let comment = content
        .len()
        .checked_sub(primary.comment.len() + 1)
        .and_then(|pos| {
            let comment = &content[pos..]; // 尝试包含前导 `;`
            comment.starts_with(';').then_some(comment)
        })
        .unwrap_or(primary.comment);
    if !comment.is_empty() {
        push_token(comment, TokenType::Comment);
    }

    tokens
}

#[derive(Clone, Copy)]
struct PrimaryToken {
    start: usize,
    end: usize,
    kind: TokenType,
}

#[derive(Clone, Copy)]
enum TokenType {
    Function,
    Variable,
    Property,
    String,
    Comment,
}

impl TokenType {
    fn to_id(self) -> u32 {
        match self {
            Self::Function => 0,
            Self::Variable => 1,
            Self::Property => 2,
            Self::String => 3,
            Self::Comment => 4,
        }
    }

    const fn all() -> &'static [SemanticTokenType] {
        const TOKEN_TYPES: &[SemanticTokenType] = &[
            SemanticTokenType::FUNCTION,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
        ];
        TOKEN_TYPES
    }
}

impl From<TokenType> for SemanticTokenType {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::Function => Self::FUNCTION,
            TokenType::Variable => Self::VARIABLE,
            TokenType::Property => Self::PROPERTY,
            TokenType::String => Self::STRING,
            TokenType::Comment => Self::COMMENT,
        }
    }
}
