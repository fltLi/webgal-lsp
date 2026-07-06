use std::{fmt, ops::Range};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NeverError;

impl fmt::Display for NeverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("不会触发的错误")
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_from_str_for_from {
    ($t:ty) => {
        impl ::std::str::FromStr for $t {
            type Err = $crate::util::NeverError;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                Ok(s.into())
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_from_str_for_serde_json {
    ($t:ty) => {
        impl ::std::str::FromStr for $t {
            type Err = ::serde_json::Error;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                ::serde_json::from_str(s)
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_display_for_serde_json {
    ($t:ty) => {
        impl ::std::fmt::Display for $t {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(&::serde_json::to_string(self).map_err(|_| ::std::fmt::Error)?)
            }
        }
    };
}

/// 避开转义 (如 `\;`) 分割字符
///
/// # Examples
/// ```
/// # use webgal_model::util::split_once_escaped;
///
/// assert_eq!(split_once_escaped("a;b", ';'), Some(("a", "b")));
/// assert_eq!(split_once_escaped("a\\b;c", ';'), Some(("a\\b", "c")));
/// assert_eq!(split_once_escaped("no semicolon", ';'), None);
/// assert_eq!(split_once_escaped("", ';'), None);
/// assert_eq!(split_once_escaped("std\\:\\:mem:hello?", ':'), Some(("std\\:\\:mem", "hello?")));
/// ```
pub fn split_once_escaped(s: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut esacped = false;
    let pos = s.char_indices().find_map(|(i, ch)| match ch {
        ch if ch == delimiter && !esacped => Some(i),
        '\\' => {
            esacped = !esacped;
            None
        }
        _ => {
            esacped = false;
            None
        }
    })?;
    Some((&s[..pos], &s[pos + 1..]))
}

/// 查找字符串中第一对匹配的开闭符区间
///
/// # Panics
/// * 当 `open == close` 时 panic, 因为无法区分开闭.
/// * 当 `open == '"'` 或 `close == '"'` 时 panic, 因为双引号已被用作字符串边界.
/// * 当 `open == '\\'` 或 `close == '\\'` 时 panic, 因为其已被用作字符串内双引号转义符.
///
/// # Behavior
/// * 忽略双引号 `"` 内的内容 (正确处理字符串内的转义 `\"`).
/// * 忽略嵌套同类型符号.
///
/// # Examples
/// ```
/// # use std::ops::Range;
///
/// # use webgal_model::util::find_closing_delimiter;
///
/// let s = "foo(bar(baz))";
/// assert_eq!(find_closing_delimiter(s, '(', ')'), Some(3..13));
///
/// let s = r#"foo("(bar)")"#;
/// assert_eq!(find_closing_delimiter(s, '(', ')'), Some(3..12)); // 忽略字符串内的括号
///
/// let s = r#""\"(bar)\"" ()"#;
/// assert_eq!(find_closing_delimiter(s, '(', ')'), Some(12..14)); // 处理字符串内的双引号转义
/// ```
pub fn find_closing_delimiter(s: &str, open: char, close: char) -> Option<Range<usize>> {
    if open == close {
        panic!("开符和闭符不能相同");
    }
    if open == '"' || close == '"' {
        panic!("开符和闭符不能是双引号");
    }
    if open == '\\' || close == '\\' {
        panic!("开符和闭符不能是转义符");
    }

    let mut in_str = false;
    let mut depth = 0;

    let mut escaped = false; // 只针对字符串内 `\"`
    let mut start = None;

    for (i, ch) in s.char_indices() {
        // 识别转义
        let last_escaped = escaped;
        escaped = ch == '\\';

        // 处理字符串状态切换
        if ch == '"' {
            if !in_str {
                in_str = true;
            } else {
                in_str = last_escaped;
            }
            continue;
        }

        // 在字符串内时忽略所有符号
        if in_str {
            continue;
        }

        if let Some(start_pos) = start {
            // 已找到开符, 处理嵌套
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(start_pos..i + 1);
                }
            }
        } else if ch == open {
            // 找到第一个开符
            start = Some(i);
            depth = 1;
        }
    }
    None
}

/// 添加分隔符写入迭代器中的元素 (末尾不加分隔符)
pub fn write_joined<W, I, T>(writer: &mut W, iter: I, sep: &str) -> fmt::Result
where
    W: fmt::Write + ?Sized,
    I: IntoIterator<Item = T>,
    T: fmt::Display,
{
    let mut iter = iter.into_iter().peekable();
    while let Some(item) = iter.next() {
        if iter.peek().is_some() {
            write!(writer, "{}{}", item, sep)?;
        } else {
            write!(writer, "{}", item)?;
        }
    }
    Ok(())
}

/// 添加分隔符自定义写入迭代器中的元素 (末尾不加分隔符)
pub fn write_joined_with<W, I, T, F>(writer: &mut W, iter: I, sep: &str, f: F) -> fmt::Result
where
    W: fmt::Write + ?Sized,
    I: IntoIterator<Item = T>,
    F: Fn(T, &mut W) -> fmt::Result,
{
    let mut iter = iter.into_iter().peekable();
    while let Some(item) = iter.next() {
        f(item, writer)?;
        if iter.peek().is_some() {
            writer.write_str(sep)?;
        }
    }
    Ok(())
}

/// 校验辅助函数, 用于聚合错误
pub(crate) fn try_validate<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut Vec<anyhow::Error>),
{
    let mut errors = Vec::new();
    f(&mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        let msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        Err(anyhow::anyhow!("验证失败: {msg}"))
    }
}

/// 依据指针获取 `needle` 字符串在 `haystack` 的对应区间
///
/// # Panics
/// 该函数假设 `needle` 是 `haystack` 的一个子串 (即内存区域完全包含在 `haystack` 内).
/// 当 `needle` 内存范围在 `haystack` 前时 panic.
///
/// # Examples
/// ```
/// # use std::ops::Range;
///
/// # use webgal_model::util::span_of;
///
/// let haystack = "Hello, world!";
/// let needle = &haystack[7..12];
/// let range: Range<usize> = span_of(haystack, needle);
/// assert_eq!(range, 7..12);
/// assert_eq!(&haystack[range], "world");
///
/// let bad_needle = "world";
/// let bad_range = span_of(haystack, bad_needle);
/// assert_ne!(bad_range, 7..12);
/// ```
pub fn span_of(haystack: &str, needle: &str) -> Range<usize> {
    let start = (needle.as_ptr() as usize)
        .checked_sub(haystack.as_ptr() as usize)
        .expect("字串需要在母串的内存内");
    start..start + needle.len()
}
