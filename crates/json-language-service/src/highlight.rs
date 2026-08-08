use std::ops;

use crate::parse::parse_for_each;

/// JSON 字符串高亮
///
/// # Behavior
/// * 键 / 字符串 - [`TokenType::String`].
/// * 数值 - [`TokenType::Number`].
/// * 布尔值 - [`TokenType::Keyword`]
pub fn highlight<F>(s: &str, mut f: F)
where
    F: FnMut(ops::Range<usize>, TokenType),
{
    parse_for_each(s, |s, span| f(span, TokenType::from_str(s)));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    Keyword,
    String,
    Number,
}

impl TokenType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        if s.starts_with('"') {
            Self::String
        } else if matches!(s, "true" | "false") {
            Self::Keyword
        } else {
            Self::Number
        }
    }
}
