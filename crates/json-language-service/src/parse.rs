//! JSON 字符串实时丢弃式词法及路径解析

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token<'a> {
    ObjectStart,
    ArrayStart,
    Key(&'a str),
    Value(&'a str),
}

fn parse_internal<'a, F>(s: &'a str, mut f: F) -> Option<Vec<Token<'a>>>
where
    F: FnMut(&'a str),
{
    let mut tokens = Vec::new();

    let mut chars = s.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        match ch {
            ' ' => {}

            // 校验并进入下一层
            '{' => {
                if !matches!(
                    tokens.last(),
                    None | Some(Token::ArrayStart) | Some(Token::Key(_))
                ) {
                    return None;
                }
                tokens.push(Token::ObjectStart);
            }
            '[' => {
                if !matches!(
                    tokens.last(),
                    None | Some(Token::ArrayStart) | Some(Token::Key(_))
                ) {
                    return None;
                }
                tokens.push(Token::ArrayStart);
            }

            // 清空当前层及可能有的前导 key
            '}' => {
                while !matches!(tokens.pop()?, Token::ObjectStart) {}
                tokens.pop_if(|token| matches!(token, Token::Key(_)));
            }
            ']' => {
                while !matches!(tokens.pop()?, Token::ArrayStart) {}
                tokens.pop_if(|token| matches!(token, Token::Key(_)));
            }

            // 将上一个 value 升级为 key
            ':' => {
                if let Some(Token::Value(value)) = tokens.pop() {
                    let key = value.strip_prefix('"')?.strip_suffix('"')?;
                    tokens.push(Token::Key(key));
                } else {
                    return None;
                }
            }

            // 出栈直到抵达边界: 空 / `{` / `[`
            ',' => {
                while !matches!(
                    tokens.last(),
                    None | Some(Token::ObjectStart) | Some(Token::ArrayStart)
                ) {
                    tokens.pop();
                }
            }

            // 识别字符串
            '"' => {
                let mut escaped = false;
                let end = chars
                    .find_map(|(j, ch)| match ch {
                        '"' if !escaped => Some(j + ch.len_utf8()),
                        '\\' => {
                            escaped = !escaped;
                            None
                        }
                        _ => {
                            escaped = false;
                            None
                        }
                    })
                    .unwrap_or(s.len()); // 走到头时循环必定结束
                let value = &s[i..end];
                f(value);
                tokens.push(Token::Value(value));
            }
            // 识别值
            _ => {
                let end = loop {
                    match chars.peek() {
                        Some(&(i, ch))
                            if matches!(ch, '{' | '}' | '[' | ']' | ':' | ',' | '"')
                                || ch.is_whitespace() =>
                        {
                            break i;
                        }
                        None => break s.len(), // 走到头时循环必定结束
                        _ => {
                            chars.next();
                        }
                    }
                };
                let value = &s[i..end];
                f(value);
                tokens.push(Token::Value(value));
            }
        }
    }

    Some(tokens)
}

/// 宽松 JSON 字符串初级词法和路径解析
///
/// # Returns
/// 从对象根抵达 JSON 字符串结尾处经过的路径.
/// 解析过程丢弃了部分不在路径上的 token, 但可能存在剩余.
pub fn parse<'a>(s: &'a str) -> Option<Vec<Token<'a>>> {
    parse_internal(s, |_| {})
}

/// 宽松 JSON 字符串初级词法解析
///
/// # Returns
/// 逐一返回 JSON 字符串中的 token 及其区间.
/// 与 [`parse`] 不同, 此函数不会丢弃任何 token.
pub fn parse_for_each<'a, F>(s: &'a str, mut f: F)
where
    F: FnMut(&'a str, Range<usize>),
{
    let start = s.as_ptr() as usize;
    parse_internal(s, |value| {
        let start = value.as_ptr() as usize - start;
        f(value, start..start + value.len());
    });
}
