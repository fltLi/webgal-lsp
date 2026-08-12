use std::ops;

use crate::parse::parse_for_each;

/// JSON 字符串高亮
///
/// # Behavior
/// * 键 / 字符串 - [`TokenType::String`].
/// * 数值 - [`TokenType::Number`].
/// * 布尔值 - [`TokenType::Keyword`].
/// * 符号 - [`TokenType::Operator`].
pub fn highlight<F>(s: &str, mut f: F)
where
    F: FnMut(ops::Range<usize>, TokenType),
{
    let mut last_end = 0;
    parse_for_each(s, |s, span| {
        // 符号
        if last_end < span.start {
            f(last_end..span.start, TokenType::Operator);
        }
        last_end = span.end;

        // token
        f(
            span,
            if s.starts_with('"') {
                TokenType::String
            } else if matches!(s, "true" | "false" | "null") {
                TokenType::Keyword
            } else {
                TokenType::Number
            },
        );
    });

    // 符号
    if last_end != s.len() {
        f(last_end..s.len(), TokenType::Operator);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    Keyword,
    String,
    Number,
    Operator,
}
