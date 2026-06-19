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
                    token_types: TOKEN_TYPES.to_vec(),
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
    for (line, sentence) in scene.sentences().iter().enumerate() {
        highlight_sentence(sentence, line, &mut tokens);
    }
    tokens
}

/// 生成一条语句的高亮
fn highlight_sentence(sentence: &SentenceInfo, line: usize, tokens: &mut Vec<SemanticToken>) {
    let SentenceInfo {
        primary, sentence, ..
    } = sentence;

    let make_token = |text, kind| {
        let span = primary.get_span(text);
        SemanticToken {
            delta_line: line as u32,
            delta_start: span.start as u32,
            length: span.len() as u32,
            token_type: token_type_id_of(&kind),
            token_modifiers_bitset: 0,
        }
    };

    // 语句类型高亮
    tokens.push(make_token(
        primary.command,
        if sentence.is_say() {
            SemanticTokenType::VARIABLE
        } else {
            SemanticTokenType::FUNCTION
        },
    ));

    // 主参数高亮
    if let Some(content) = primary.content
        && sentence.is_say()
    {
        tokens.push(make_token(content, SemanticTokenType::STRING));
    }

    // 参数高亮
    tokens.extend(
        primary
            .arguments
            .iter()
            .map(|&(name, _)| make_token(name, SemanticTokenType::PROPERTY)),
    );

    // 注释高亮
    if !primary.comment.is_empty() {
        tokens.push(make_token(primary.comment, SemanticTokenType::COMMENT));
    }
}

const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
];

fn token_type_id_of(token_type: &SemanticTokenType) -> u32 {
    TOKEN_TYPES
        .iter()
        .enumerate()
        .find_map(|(id, kind)| (kind == token_type).then_some(id))
        .unwrap() as u32
}
