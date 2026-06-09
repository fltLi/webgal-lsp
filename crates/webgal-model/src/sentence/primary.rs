//! 语句词法解析

use std::{fmt, ops::Range, slice};

use crate::util::span_of;

/// 初级语句
///
/// 该阶段仅做词法级解析, 不验证语句是否合法 / 参数是否完备.
/// 若要获得类型安全的 [`Sentence`] 枚举变体, 请使用 [`FromPrimary`] 进一步解析.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimarySentence<'a> {
    pub command: &'a str,
    pub content: Option<&'a str>,
    pub arguments: Vec<(&'a str, Option<&'a str>)>,
    pub comment: &'a str,
}

impl<'a> PrimarySentence<'a> {
    /// 语句词法解析
    ///
    /// # Performance
    /// 最坏扫描 4 遍整条语句.
    /// 仅在收集参数数组时使用堆分配.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(line: &'a str) -> Self {
        // 分离注释
        let (line, comment) = split_comment(line).unwrap_or((line, ""));

        // 提取语句头 (后续判定为语句类型或主参数)
        let (command, line) = match line.split_once(':') {
            Some((command, line)) => (Some(command), line),
            None => (None, line),
        };

        // 切分参数
        let mut argument_split = ArgumentSplit::new(line).peekable();
        let content = argument_split.next().unwrap();

        // 整理语句类型和主参数
        let (command, content) =
            if command.is_none() && !content.is_empty() && argument_split.peek().is_none() {
                (line, None) // 形如 `content ;` / `content `, 会被识别为对话
            } else {
                match command {
                    Some(command) => (command, Some(content.trim())),
                    None => (content, None),
                }
            };

        // 收集参数
        let arguments: Vec<_> = argument_split
            .map(|argument| match argument.split_once('=') {
                Some((name, value)) => (name, Some(value.trim())),
                None => (argument, None),
            })
            .collect();

        Self {
            command,
            content,
            arguments,
            comment,
        }
    }

    /// 依据名称遍历查找参数
    pub fn get_argument(&self, name: &str) -> Option<(usize, Option<&'a str>)> {
        self.arguments
            .iter()
            .enumerate()
            .find_map(|(i, &(k, v))| (k == name).then_some((i, v)))
    }

    /// 获取参数完整字符串
    ///
    /// # Panics
    /// 当参数索引越界时 panic.
    ///
    /// # Safety
    /// 确保参数键值位于同一字符串.
    /// 对于由 [`Self::from_str`] 构造的语句, 此操作安全性已保证.
    pub fn get_full_argument(&self, index: usize) -> &'a str {
        // 处理 bool 参数语法糖
        let argument = match self.arguments[index] {
            (name, Some(value)) => {
                let start = name.as_ptr();
                let end = unsafe { value.as_ptr().add(value.len()) };
                let len = end as usize - start as usize;
                unsafe { str::from_utf8_unchecked(slice::from_raw_parts(start, len)) }
            }
            (name, None) => name,
        };

        // 补充参数标头
        let start = unsafe { argument.as_ptr().sub(1) };
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(start, argument.len() + 1)) }
    }

    /// 依据指针获取字符串在语句的对应区间
    ///
    /// # Panics
    /// 该函数假设提供的字符串是语句原始字符串的一个子串 (即内存区域完全包含在其内).
    /// 当提供字符串在语句原始字符串前时 panic.
    ///
    /// # Behavior
    /// * 此函数假设 `command` 与语句原始字符串开头对齐, 且提供的字符串不超过其末尾.
    pub fn get_span(&self, s: &str) -> Range<usize> {
        span_of(self.command, s)
    }
}

impl<'a> fmt::Display for PrimarySentence<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.command)?;

        if let Some(content) = self.content {
            write!(f, ":{content}")?;
        }

        for &(name, value) in &self.arguments {
            match value {
                Some(value) => write!(f, " -{name}={value}")?,
                None => write!(f, " -{name}")?,
            }
        }

        write!(f, ";{}", self.comment)
    }
}

/// 在第一个未转义 (`\;`) 的 `;` 处分割语句与注释
pub fn split_comment(line: &str) -> Option<(&str, &str)> {
    let mut esacped = false;
    let pos = line.char_indices().find_map(|(i, ch)| match ch {
        ';' if !esacped => Some(i),
        '\\' => {
            esacped = !esacped;
            None
        }
        _ => {
            esacped = false;
            None
        }
    })?;
    Some((&line[..pos], &line[pos + 1..]))
}

/// 语句参数分割迭代器
///
/// # Behavior
/// * 确保第一次解析结果非空 (为了让 content 为空串时其内存地址也保持原样).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ArgumentSplit<'a> {
    inner: Option<&'a str>,
}

impl<'a> ArgumentSplit<'a> {
    pub fn new(line: &'a str) -> Self {
        Self { inner: Some(line) }
    }

    /// 查找下一个 `{whitespace}-` 参数分割位置
    fn find_next_delimiter(&self) -> Option<usize> {
        let mut last_is_whitespace = false;
        self.inner?.char_indices().find_map(|(i, ch)| match ch {
            '-' if last_is_whitespace => Some(i - 1),
            ch => {
                last_is_whitespace = ch.is_whitespace();
                None
            }
        })
    }
}

impl<'a> Iterator for ArgumentSplit<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.inner?;

        let (argument, remain) = match self.find_next_delimiter() {
            Some(pos) => (&line[..pos], Some(&line[pos + 2..])),
            None => (line, None),
        };

        self.inner = remain;
        Some(argument.trim_end())
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    // -------- 注释分割 --------

    #[test]
    fn split_comment_basic() {
        assert_eq!(split_comment("a;b"), Some(("a", "b")));
        assert_eq!(split_comment(r"a\;b;c"), Some(("a\\;b", "c")));
        assert_eq!(split_comment("no semicolon"), None);
        assert_eq!(split_comment(""), None);
    }

    // -------- 参数分割 --------

    #[test]
    fn argument_split_works() {
        let mut split = ArgumentSplit::new(" -a=b -c -d=e");
        assert_eq!(split.next(), Some(""));
        assert_eq!(split.next(), Some("a=b"));
        assert_eq!(split.next(), Some("c"));
        assert_eq!(split.next(), Some("d=e"));
        assert_eq!(split.next(), None);
    }

    #[test]
    fn argument_split_empty() {
        let mut split = ArgumentSplit::new("");
        assert_eq!(split.next(), Some(""));
    }

    // -------- 基础解析 --------

    #[test]
    fn parse_basic_command_with_content_and_arguments() {
        let s = "changeBg:bg.png -next -unlockname=home; comment";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "changeBg");
        assert_eq!(parsed.content, Some("bg.png"));
        assert_eq!(
            parsed.arguments,
            vec![("next", None), ("unlockname", Some("home"))]
        );
        assert_eq!(parsed.comment, " comment");
        assert_eq!(parsed.to_string(), s);
    }

    #[test]
    fn parse_no_command() {
        let s = ":some content -flag; comment";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "");
        assert_eq!(parsed.content, Some("some content"));
        assert_eq!(parsed.arguments, vec![("flag", None)]);
        assert_eq!(parsed.comment, " comment");
    }

    #[test]
    fn parse_no_content_no_arguments() {
        let s = "pixiInit;";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "pixiInit");
        assert_eq!(parsed.content, None);
        assert!(parsed.arguments.is_empty());
        assert_eq!(parsed.comment, "");
    }

    #[test]
    fn parse_escaped_semicolon_in_style() {
        let s = r#"chara:[text](style=color:#FFFFFF\; ruby=ruby) -id=chara;"#;
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "chara");
        assert_eq!(
            parsed.content,
            Some(r#"[text](style=color:#FFFFFF\; ruby=ruby)"#)
        );
        assert_eq!(parsed.arguments, vec![("id", Some("chara"))]);
        assert_eq!(parsed.comment, "");
    }

    #[test]
    fn parse_empty_content() {
        let s = "cmd: -flag";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "cmd");
        assert_eq!(parsed.content, Some(""));
        assert_eq!(parsed.arguments, vec![("flag", None)]);
        assert_eq!(parsed.comment, "");
    }

    #[test]
    fn parse_no_colon_fallback() {
        let s = "commandWithoutColon -flag; comment";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "commandWithoutColon");
        assert_eq!(parsed.content, None);
        assert_eq!(parsed.arguments, vec![("flag", None)]);
        assert_eq!(parsed.comment, " comment");
    }

    #[test]
    fn parse_command_with_trailing_space() {
        let s = "commandWithSpace ;";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "commandWithSpace ");
        assert_eq!(parsed.content, None);
        assert!(parsed.arguments.is_empty());
        assert_eq!(parsed.comment, "");
    }

    #[test]
    fn parse_malformed_with_hyphen_in_content() {
        let s = r#"setTransform:{" -what?"}; -unreachable"#;
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "setTransform");
        assert_eq!(parsed.content, Some(r#"{""#));
        assert_eq!(parsed.arguments, vec![(r#"what?"}"#, None)]);
        assert_eq!(parsed.comment, " -unreachable");
    }

    // -------- 空格处理 --------

    #[test]
    fn parse_content_trim() {
        let s = "cmd: \t content with spaces   -arg=val;";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.content, Some("content with spaces"));
    }

    #[test]
    fn parse_argument_name_preserves_spaces_value_trim() {
        let s = "cmd:content -\n arg name   =  value with spaces  ;";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.arguments[0].0, "\n arg name   ");
        assert_eq!(parsed.arguments[0].1, Some("value with spaces"));
    }

    #[test]
    fn parse_command_trailing_space() {
        let s = "commandWithSpace \t ;";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.command, "commandWithSpace \t ");
    }

    // -------- 参数查找 --------

    #[test]
    fn get_argument_by_name() {
        let s = "cmd:content -a=1 -b -c=3";
        let parsed = PrimarySentence::from_str(s);
        assert_eq!(parsed.get_argument("a"), Some((0, Some("1"))));
        assert_eq!(parsed.get_argument("b"), Some((1, None)));
        assert_eq!(parsed.get_argument("c"), Some((2, Some("3"))));
        assert_eq!(parsed.get_argument("d"), None);
    }

    #[test]
    fn get_full_argument_string() {
        let s = "cmd:content -flag1=value1 -flag2";
        let parsed = PrimarySentence::from_str(s);
        let full = parsed.get_full_argument(0);
        assert_eq!(full, "-flag1=value1");
        let full2 = parsed.get_full_argument(1);
        assert_eq!(full2, "-flag2");
    }

    // -------- 区间定位 --------

    #[test]
    fn get_span_of_argument() {
        let s = "cmd:content -flag=value";
        let parsed = PrimarySentence::from_str(s);
        let (idx, _) = parsed.get_argument("flag").unwrap();
        let arg_str = parsed.get_full_argument(idx);
        let span = parsed.get_span(arg_str);
        assert_eq!(&s[span.clone()], "-flag=value");
        assert_eq!(span.start, s.find('-').unwrap());
        assert_eq!(span.end, s.len());
    }

    // -------- 解析往返 --------

    #[test]
    fn roundtrip_typical_sentences() {
        let samples = vec![
            "changeFigure:model.json -left -id=chara -next; comment",
            "setTransform:{\"position\":{\"x\":400}} -target=bg-main -keep -next; TODO: ...",
            r#"chara:[content](style=color:#FFFFFF\; ruby=ruby) -figureId=chara;"#,
            "pixiInit;",
            ":;",
        ];
        for original in samples {
            let parsed = PrimarySentence::from_str(original);
            assert_eq!(parsed.to_string(), original);
        }
    }

    #[test]
    fn roundtrip_location_example() {
        let original = "changeBg:bg.png -next -unlockname=home;";
        let parsed = PrimarySentence::from_str(original);
        let (index, value) = parsed.get_argument("unlockname").unwrap();
        let argument = parsed.get_full_argument(index);
        let span = parsed.get_span(argument);
        assert_eq!(value, Some("home"));
        assert_eq!(argument, "-unlockname=home");
        assert_eq!(span, 22..38);
        assert_eq!(parsed.to_string(), original);
    }
}
