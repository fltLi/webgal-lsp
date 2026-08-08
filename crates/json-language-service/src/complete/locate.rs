use crate::parse::{Token, parse};

/// JSON 字符串输入位置信息
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location<'a> {
    pub path: Vec<Node<'a>>,
    pub ident: Ident<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node<'a> {
    Field(&'a str),
    Array,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident<'a> {
    Key(&'a str),
    Value(&'a str),
    #[default]
    Other,
}

impl<'a> Location<'a> {
    /// 宽松解析 JSON 字符串 (末尾) 对应的位置信息
    pub fn locate(s: &'a str) -> Self {
        let mut tokens = match parse(s) {
            Some(tokens) if !tokens.is_empty() => tokens,
            _ => return Self::default(),
        };
        let (ident, key) = ident_and_key_of(s, &tokens);
        tokens.pop_if(|token| matches!(token, Token::Key(_) | Token::Value(_)));
        let mut path = path_of(&tokens[..tokens.len()]);
        if let Some(key) = key {
            path.push(Node::Field(key));
        }
        Self { path, ident }
    }
}

fn ident_and_key_of<'a>(s: &'a str, tokens: &[Token<'a>]) -> (Ident<'a>, Option<&'a str>) {
    match tokens.last().unwrap() {
        // 位于 `[` / `{` / `,` 后, 或者为空
        Token::ObjectStart => (Ident::Key(""), None),
        Token::ArrayStart => (Ident::Value(""), None),

        // 期望输入 key 对应的 value
        Token::Key(key) => (Ident::Value(""), Some(key)),

        Token::Value(value) if !value.ends_with('"') || *value == "\"" => {
            let value = suffix_from_sub(s, value).unwrap();

            // 属于前一个 key 的 value
            if tokens.len() >= 2 && matches!(tokens[tokens.len() - 2], Token::Key(_)) {
                return (Ident::Value(value), None);
            }

            if let Some(kind) = tokens
                .iter()
                .rfind(|token| matches!(token, Token::ObjectStart | Token::ArrayStart))
            {
                match kind {
                    // 位于 object 中, 且为输入中字符串, 升级为 key
                    Token::ObjectStart if let Some(v) = value.strip_prefix('"') => {
                        (Ident::Key(v), None)
                    }
                    // 位于 array 中, 视为一项 value
                    Token::ArrayStart => (Ident::Value(value), None),
                    _ => (Ident::Other, None),
                }
            } else {
                (Ident::Other, None)
            }
        }

        _ => (Ident::Other, None),
    }
}

fn path_of<'a>(tokens: &[Token<'a>]) -> Vec<Node<'a>> {
    let mut accept_key = false;
    tokens
        .iter()
        .rev() // 倒序生成路径段
        .filter_map(|&token| match token {
            Token::ObjectStart => {
                accept_key = true;
                None
            }
            Token::ArrayStart => {
                accept_key = true;
                Some(Node::Array)
            }

            Token::Key(key) if accept_key => {
                accept_key = false;
                Some(Node::Field(key))
            }
            _ => {
                accept_key = false;
                None
            }
        })
        .rev() // 正向输出路径段
        .collect()
}

fn suffix_from_sub<'a>(s: &'a str, sub: &str) -> Option<&'a str> {
    let start = (sub.as_ptr() as usize).checked_sub(s.as_ptr() as usize)?;
    s.get(start..)
}
