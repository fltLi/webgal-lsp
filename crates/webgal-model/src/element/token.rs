//! 文本拓展语法解析

use std::{
    fmt::{self, Write},
    mem,
};

use derive_more::{Deref, DerefMut, From, Into};

use crate::util::write_joined_with;

/// 带扩展文本
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token<'a> {
    pub text: &'a str,
    pub ruby: &'a str, // 空串表示无注音
    pub style: Vec<(&'a str, Vec<(&'a str, &'a str)>)>,
}

impl<'a> Token<'a> {
    pub fn with_text(text: &'a str) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }

    /// 解析文本和样式字符串 (可包含注音)
    ///
    /// # Behavior
    /// * **不规范文本拓展**的解析与 WebGAL 实际行为不符.
    /// * 样式组会进行排序, 组内参数会进行排序和去重 (重复参数保留最后加入的).
    ///
    /// # Performance
    /// 最坏扫描 3 遍整条字符串, 以及进行一个参数排序和去重.
    /// 仅在收集参数数组时发生堆分配.
    pub fn with_text_and_style(text: &'a str, style: &'a str) -> Self {
        // 例如: `g1=args1...\; g2=args2...` 会被分割为 [`g1`, `args1...\; g2`, `args2...`].
        // 对于分割非首项, 其样式组名称位于上一项 (末尾), 需要分割上一项 (但首项不用分割).
        let mut style_split = style.split('=').peekable();
        let mut group = style_split.next().unwrap().trim(); // split 总有至少一个值

        // 不存在 `=` 分割的样式组, 整体视为注音
        if style_split.peek().is_none() {
            return Self {
                text,
                ruby: style,
                style: Vec::new(),
            };
        }

        let mut ruby = None;
        let mut style: Vec<(&str, Vec<_>)> = Vec::new();

        while let Some(arguments) = style_split.next() {
            let (current_arguments, next_group) = if style_split.peek().is_some() {
                // 获取下一个样式组名称
                arguments.rsplit_once("\\;").unwrap_or(("", arguments))
            } else {
                (arguments, "") // 最后一组末尾不包含样式组名称
            };

            // 处理 `rubu=xxx`
            if group == "ruby" {
                ruby.get_or_insert(current_arguments.trim()); // 注音取第一个出现的
                continue;
            }

            // 二分查找 / 创建样式组
            let (_, arguments) = match style.binary_search_by_key(&group, |&(probe, _)| probe) {
                Ok(index) => &mut style[index],
                Err(index) => style.insert_mut(index, (group, Vec::new())),
            };

            // 加入样式参数, 形如 `color:#FF8899`
            arguments.extend(
                current_arguments
                    .split("\\;")
                    .map(|argument| argument.split_once(':').unwrap_or((argument, "")))
                    .map(|(name, value)| (name.trim(), value.trim())),
            );

            // 设置下一个样式组名称
            group = next_group.trim();
        }

        // 样式组排序, 去重时取最后一个出现的
        for (_, arguments) in &mut style {
            arguments.reverse();
            arguments.sort_by_key(|&(a, _)| a);
            arguments.dedup_by(|(a, _), (b, _)| a == b);
        }

        Self {
            text,
            ruby: ruby.unwrap_or_default(),
            style,
        }
    }

    /// 查找样式组
    ///
    /// # Behavior
    /// * 此函数假设样式组有序.
    pub fn get_style_group(&self, group: &str) -> Option<&[(&'a str, &'a str)]> {
        self.style
            .binary_search_by_key(&group, |&(probe, _)| probe)
            .ok()
            .map(|index| self.style[index].1.as_slice())
    }

    /// 查找样式
    ///
    /// # Behavior
    /// * 此函数假设样式组及其内部参数有序.
    pub fn get_style(&self, group: &str, name: &str) -> Option<&'a str> {
        let arguments = self.get_style_group(group)?;
        arguments
            .binary_search_by_key(&name, |&(probe, _)| probe)
            .ok()
            .map(|index| arguments[index].1)
    }
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self {
                text,
                ruby: "",
                style,
            } if style.is_empty() => f.write_str(text),

            Self { text, ruby, style } if style.is_empty() => write!(f, "[{text}]({ruby})"),

            Self { text, ruby, style } => {
                write!(f, "[{text}](")?;
                write_joined_with(f, style.iter(), "\\; ", |(group, arguments), f| {
                    write!(f, "{group}=")?;
                    write_joined_with(f, arguments.iter(), "\\;", |(name, value), f| {
                        write!(f, "{name}:{value}")
                    })
                })?;
                if !ruby.is_empty() {
                    write!(f, "\\; ruby={ruby}")?;
                }
                f.write_char(')')
            }
        }
    }
}

/// 连续带扩展文本
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, From, Into, Deref, DerefMut)]
pub struct TokenList<'a>(Vec<Token<'a>>);

impl<'a> TokenList<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'a str) -> Self {
        Self(TokenSplit::new(s).collect())
    }
}

impl fmt::Display for TokenList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.iter().try_for_each(|token| token.fmt(f))
    }
}

/// 带扩展文本分割与解析
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct TokenSplit<'a> {
    inner: &'a str,
    pending: Option<(&'a str, &'a str)>,
}

impl<'a> TokenSplit<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            inner: text,
            pending: None,
        }
    }
}

impl<'a> Iterator for TokenSplit<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // 解析上次提前捕获的带拓展 token
        if let Some((text, style)) = self.pending.take() {
            return Some(Token::with_text_and_style(text, style));
        }

        if self.inner.is_empty() {
            return None;
        }

        // 尝试解析为 `current[token-text](token-style)remain` 的格式
        let (current, token, remain) = match (|| {
            let (current, s) = self.inner.split_once('[')?;
            let (text, s) = s.split_once("](")?;
            let (style, remain) = s.split_once(')')?;
            Some((current, (text, style), remain))
        })() {
            Some(v) => v,
            // 解析失败时将全部字符串视为普通文本
            None => return Some(Token::with_text(mem::take(&mut self.inner))),
        };

        self.inner = remain;
        self.pending = Some(token);
        Some(Token::with_text(current))
    }
}

impl<'a> From<&'a str> for TokenSplit<'a> {
    fn from(value: &'a str) -> Self {
        TokenSplit::new(value)
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    // -------- Token 基础 --------

    #[test]
    fn token_basic_plain_text() {
        let token = Token::with_text("Hello");
        assert_eq!(token.text, "Hello");
        assert!(token.ruby.is_empty());
        assert!(token.style.is_empty());
    }

    #[test]
    fn token_basic_ruby_only() {
        let token = Token::with_text_and_style("漢字", "ふりがな");
        assert_eq!(token.text, "漢字");
        assert_eq!(token.ruby, "ふりがな");
        assert!(token.style.is_empty());
    }

    #[test]
    fn token_basic_single_style_group() {
        let token = Token::with_text_and_style("文字", "style=color:red");
        assert_eq!(token.text, "文字");
        assert_eq!(token.ruby, "");
        assert_eq!(token.style.len(), 1);
        assert_eq!(token.style[0].0, "style");
        assert_eq!(token.style[0].1, &[("color", "red")]);
    }

    #[test]
    fn token_basic_style_with_ruby() {
        let token = Token::with_text_and_style("漢字", "style=color:red\\; ruby=かんじ");
        assert_eq!(token.ruby, "かんじ");
        let style = token.get_style_group("style").unwrap();
        assert_eq!(style, &[("color", "red")]);
    }

    // -------- 样式解析: 去重与排序 --------

    #[test]
    fn token_style_argument_deduplication() {
        let token = Token::with_text_and_style("test", "style=color:red\\;color:blue");
        let args = &token.style[0].1;
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], ("color", "blue"));
    }

    #[test]
    fn token_style_argument_sorting() {
        let token = Token::with_text_and_style("test", "style=z:1\\;a:2\\;m:3");
        let args = &token.style[0].1;
        assert_eq!(args, &[("a", "2"), ("m", "3"), ("z", "1")]);
    }

    #[test]
    fn token_style_group_sorting() {
        let token = Token::with_text_and_style("test", "z=0\\; a=1\\; m=2");
        let groups: Vec<_> = token.style.iter().map(|(g, _)| *g).collect();
        assert_eq!(groups, vec!["a", "m", "z"]);
    }

    // -------- TokenSplit 迭代器 --------

    #[test]
    fn token_split_plain_text() {
        let tokens: Vec<_> = TokenSplit::new("ただのテキスト").collect();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "ただのテキスト");
    }

    #[test]
    fn token_split_simple_ruby() {
        let s = "これは[注釈](ちゅうしゃく)の例です。";
        let tokens: Vec<_> = TokenSplit::new(s).collect();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "これは");
        assert_eq!(tokens[1].text, "注釈");
        assert_eq!(tokens[1].ruby, "ちゅうしゃく");
        assert_eq!(tokens[2].text, "の例です。");
    }

    #[test]
    fn token_split_consecutive_tokens() {
        let s = "[A](a)[B](b)[C](c)";
        let tokens: Vec<_> = TokenSplit::new(s).collect();
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[1].text, "A");
        assert_eq!(tokens[1].ruby, "a");
        assert_eq!(tokens[3].text, "B");
        assert_eq!(tokens[3].ruby, "b");
        assert_eq!(tokens[5].text, "C");
        assert_eq!(tokens[5].ruby, "c");
    }

    #[test]
    fn token_split_unclosed_token() {
        let s = "[未閉じ";
        let tokens: Vec<_> = TokenSplit::new(s).collect();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, s);
    }

    // -------- TokenList --------

    #[test]
    fn token_list_roundtrip() {
        let original = "[漢字](かんじ)と[スタイル](style=color:red)のテスト";
        let list = TokenList::from_str(original);
        let reconstructed = list.to_string();
        let reparsed = TokenList::from_str(&reconstructed);
        assert_eq!(list.len(), reparsed.len());
        for (a, b) in list.iter().zip(reparsed.iter()) {
            assert_eq!(a.text, b.text);
            assert_eq!(a.ruby, b.ruby);
            assert_eq!(a.style, b.style);
        }
    }

    // -------- get_style_group / get_style --------

    #[test]
    fn token_get_style_group_returns_correct_group() {
        let token = Token::with_text_and_style(
            "text",
            "style=color:red\\;size:large\\; style-alltext=font-style:italic",
        );
        let style_group = token.get_style_group("style").unwrap();
        assert_eq!(style_group, &[("color", "red"), ("size", "large")]);

        let alltext_group = token.get_style_group("style-alltext").unwrap();
        assert_eq!(alltext_group, &[("font-style", "italic")]);
    }

    #[test]
    fn token_get_style_group_returns_none_for_missing_group() {
        let token = Token::with_text_and_style("text", "style=color:red");
        assert_eq!(token.get_style_group("missing"), None);
        assert_eq!(
            token.get_style_group("style"),
            Some(&[("color", "red")] as &[_])
        );
    }

    #[test]
    fn token_get_style_returns_correct_value() {
        let token = Token::with_text_and_style("text", "style=color:red\\;size:large\\; ruby=ふり");
        assert_eq!(token.get_style("style", "color"), Some("red"));
        assert_eq!(token.get_style("style", "size"), Some("large"));
        assert_eq!(token.get_style("style", "ruby"), None);
    }

    #[test]
    fn token_get_style_returns_none_for_missing_style_name() {
        let token = Token::with_text_and_style("text", "style=color:red");
        assert_eq!(token.get_style("style", "missing"), None);
    }

    #[test]
    fn token_get_style_works_with_multiple_groups() {
        let token = Token::with_text_and_style("text", "group1=a:1\\;b:2\\; group2=x:9");
        assert_eq!(token.get_style("group1", "a"), Some("1"));
        assert_eq!(token.get_style("group1", "b"), Some("2"));
        assert_eq!(token.get_style("group2", "x"), Some("9"));
        assert_eq!(token.get_style("group2", "y"), None);
    }
}
