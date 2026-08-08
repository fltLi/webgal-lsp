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
        let mut tokens = match Token::parse(s) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Token<'a> {
    ObjectStart,
    ArrayStart,
    Key(&'a str),
    Value(&'a str),
}

impl<'a> Token<'a> {
    /// 宽松 JSON 字符串初级解析, 返回抵达路径 (不保证路径为单链)
    pub fn parse(s: &'a str) -> Option<Vec<Self>> {
        let mut tokens = Vec::new();

        let mut chars = s.char_indices().peekable();
        while let Some((i, ch)) = chars.next() {
            match ch {
                ' ' => {}

                // 校验并进入下一层
                '{' => {
                    if !matches!(
                        tokens.last(),
                        None | Some(Self::ArrayStart) | Some(Self::Key(_))
                    ) {
                        return None;
                    }
                    tokens.push(Self::ObjectStart);
                }
                '[' => {
                    if !matches!(
                        tokens.last(),
                        None | Some(Self::ArrayStart) | Some(Self::Key(_))
                    ) {
                        return None;
                    }
                    tokens.push(Self::ArrayStart);
                }

                // 清空当前层及可能有的前导 key
                '}' => {
                    while !matches!(tokens.pop()?, Self::ObjectStart) {}
                    tokens.pop_if(|token| matches!(token, Self::Key(_)));
                }
                ']' => {
                    while !matches!(tokens.pop()?, Self::ArrayStart) {}
                    tokens.pop_if(|token| matches!(token, Self::Key(_)));
                }

                // 将上一个 value 升级为 key
                ':' => {
                    if let Some(Self::Value(value)) = tokens.pop() {
                        let key = value.strip_prefix('"')?.strip_suffix('"')?;
                        tokens.push(Self::Key(key));
                    } else {
                        return None;
                    }
                }

                // 出栈直到抵达边界: 空 / `{` / `[`
                ',' => {
                    while !matches!(
                        tokens.last(),
                        None | Some(Self::ObjectStart) | Some(Self::ArrayStart)
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
                    tokens.push(Self::Value(&s[i..end]));
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
                    tokens.push(Self::Value(&s[i..end]));
                }
            }
        }

        Some(tokens)
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
