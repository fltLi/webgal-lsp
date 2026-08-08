use lsp_types::*;

use crate::parse::parse_for_each;

/// JSON 字符串高亮
///
/// # Behavior
/// * 键 / 字符串 - [`SemanticTokenType::STRING`].
/// * 数值 - [`SemanticTokenType::NUMBER`].
/// * 布尔值 - [`SemanticTokenType::KEYWORD`]
pub fn highlight<F>(s: &str, mut token_type_id_of: F) -> Vec<SemanticToken>
where
    F: FnMut(SemanticTokenType) -> u32,
{
    let mut tokens = Vec::new();
    let mut last_end = 0;

    parse_for_each(s, |token, span| {
        let kind = token_type_of(token);
        let delta_start = (span.start - last_end) as u32;
        last_end = span.end;

        tokens.push(SemanticToken {
            delta_line: 0,
            delta_start,
            length: span.len() as u32,
            token_type: token_type_id_of(kind),
            token_modifiers_bitset: 0,
        });
    });

    tokens
}

fn token_type_of(s: &str) -> SemanticTokenType {
    if s.starts_with('"') {
        SemanticTokenType::STRING
    } else if s == "true" || s == "false" {
        SemanticTokenType::KEYWORD
    } else {
        SemanticTokenType::NUMBER
    }
}
