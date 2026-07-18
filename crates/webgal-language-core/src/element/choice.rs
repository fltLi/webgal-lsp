//! 分支选项

use std::fmt;

#[cfg(feature = "serde")]
use serde::Serialize;

use crate::util::{find_closing_delimiter, split_once_escaped};

/// 分支选项
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize), serde(rename_all = "camelCase"))]
pub struct Choice {
    pub prompt: String,
    pub target: Option<String>,
    // 控制
    pub show: Option<String>,
    pub enable: Option<String>,
}

impl Choice {
    /// 解析选项字符串
    ///
    /// # Behavior
    /// * 移除所有 `\|`, `\:` 转义.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        ChoiceView::from_str(s).to_choice()
    }

    /// 将分支选项原始视图转换为所有权选项
    ///
    /// # Behavior
    /// * 移除所有 `\|`, `\:` 转义.
    pub fn from_view(view: &ChoiceView) -> Self {
        let ChoiceView {
            prompt,
            target,
            show,
            enable,
        } = view;
        Self {
            prompt: unescape_text(prompt),
            target: target.map(unescape_text),
            show: show.map(unescape_text),
            enable: enable.map(unescape_text),
        }
    }

    pub fn has_condition(&self) -> bool {
        self.show.is_some() || self.enable.is_some()
    }
}

impl fmt::Display for Choice {
    /// 打印分支选项
    ///
    /// # Behavior
    /// * 自动添加 `\|`, `\:` 转义.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.has_condition() {
            if let Some(show) = &self.show {
                write!(f, "({})", escape_text(show))?;
            }
            if let Some(enable) = &self.enable {
                write!(f, "[{}]", escape_text(enable))?;
            }
            f.write_str("->")?;
        }

        write!(f, "{}", escape_text(&self.prompt))?;
        if let Some(target) = &self.target {
            write!(f, ":{}", escape_text(target))?;
        }
        Ok(())
    }
}

impl From<ChoiceView<'_>> for Choice {
    fn from(value: ChoiceView) -> Self {
        Self::from_view(&value)
    }
}

/// 分支选项原始视图
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChoiceView<'a> {
    pub prompt: &'a str,
    pub target: Option<&'a str>,
    // 控制
    pub show: Option<&'a str>,
    pub enable: Option<&'a str>,
}

impl<'a> ChoiceView<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'a str) -> Self {
        let s = s.trim();

        let ((show, enable), body) = match s.split_once("->") {
            Some((condition, body)) => (parse_condition(condition), body),
            None => ((None, None), s),
        };
        let (prompt, target) = parse_prompt_and_target(body);

        Self {
            prompt,
            target,
            show,
            enable,
        }
    }

    pub fn has_condition(&self) -> bool {
        self.show.is_some() || self.enable.is_some()
    }

    /// 将分支选项原始视图转换为所有权选项
    ///
    /// # Behavior
    /// * 移除所有 `\|`, `\:` 转义.
    pub fn to_choice(&self) -> Choice {
        Choice::from_view(self)
    }
}

impl fmt::Display for ChoiceView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.has_condition() {
            if let Some(show) = self.show {
                write!(f, "({show})")?;
            }
            if let Some(enable) = self.enable {
                write!(f, "[{enable}]")?;
            }
            f.write_str("->")?;
        }

        f.write_str(self.prompt)?;
        if let Some(target) = self.target {
            write!(f, ":{target}")?;
        }
        Ok(())
    }
}

fn parse_condition(s: &str) -> (Option<&str>, Option<&str>) {
    let show_start_hint = s.find('(');
    let enable_start_hint = s.find('[');

    match (show_start_hint, enable_start_hint) {
        (Some(show_start), Some(enable_start)) if show_start < enable_start => {
            let show_span = find_closing_delimiter(&s[show_start..], '(', ')');
            let show = show_span
                .as_ref()
                .map(|range| &s[show_start + range.start + 1..show_start + range.end - 1]);
            let enable = show_span.and_then(|show_range| {
                let show_end = show_start + show_range.end;
                find_closing_delimiter(&s[show_end..], '[', ']')
                    .map(|range| &s[show_end + range.start + 1..show_end + range.end - 1])
            });
            (show, enable)
        }

        (Some(_), Some(enable_start)) => {
            let enable_span = find_closing_delimiter(&s[enable_start..], '[', ']');
            let enable = enable_span
                .as_ref()
                .map(|range| &s[enable_start + range.start + 1..enable_start + range.end - 1]);
            let show = enable_span.and_then(|enable_range| {
                let enable_end = enable_start + enable_range.end;
                find_closing_delimiter(&s[enable_end..], '(', ')')
                    .map(|range| &s[enable_end + range.start + 1..enable_end + range.end - 1])
            });
            (show, enable)
        }

        (Some(show), None) => {
            let show = find_closing_delimiter(&s[show..], '(', ')')
                .map(|range| &s[show + range.start + 1..show + range.end - 1]);
            (show, None)
        }

        (None, Some(enable)) => {
            let enable = find_closing_delimiter(&s[enable..], '[', ']')
                .map(|range| &s[enable + range.start + 1..enable + range.end - 1]);
            (None, enable)
        }

        (None, None) => (None, None),
    }
}

fn parse_prompt_and_target(s: &str) -> (&str, Option<&str>) {
    match split_once_escaped(s, ':') {
        Some((prompt, target)) => (prompt, Some(target)),
        None => (s, None),
    }
}

/// 选项分割迭代器
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ChoiceSplit<'a> {
    inner: &'a str,
}

impl<'a> ChoiceSplit<'a> {
    pub fn new(s: &'a str) -> Self {
        Self { inner: s.trim() }
    }
}

impl<'a> From<&'a str> for ChoiceSplit<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl<'a> Iterator for ChoiceSplit<'a> {
    type Item = ChoiceView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_empty() {
            return None;
        }

        let (choice, remain) = split_once_escaped(self.inner, '|').unwrap_or((self.inner, ""));
        self.inner = remain.trim_start();

        Some(ChoiceView::from_str(choice))
    }
}

/// 转义字符串中的 `|` 和 `:` 为 `\|` 和 `\:`
fn escape_text(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '|' => escaped.push_str("\\|"),
            ':' => escaped.push_str("\\:"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// 解转义字符串中的 `\|` 和 `\:` 为 `|` 和 `:`
fn unescape_text(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.peek() {
                match next {
                    '|' => {
                        result.push('|');
                        chars.next();
                    }
                    ':' => {
                        result.push(':');
                        chars.next();
                    }
                    _ => result.push(ch),
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    // -------- 基础解析与序列化 --------

    #[test]
    fn choice_basic_no_condition() {
        let s = "prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert_eq!(choice.show, None);
        assert_eq!(choice.enable, None);
        assert!(!choice.has_condition());
        assert_eq!(choice.to_string(), s);
    }

    #[test]
    fn choice_prompt_only() {
        let s = "ただのプロンプト";
        let choice = Choice::from_str(s);
        assert_eq!(choice.prompt, "ただのプロンプト");
        assert_eq!(choice.target, None);
        assert_eq!(choice.show, None);
        assert_eq!(choice.enable, None);
        assert!(!choice.has_condition());
        assert_eq!(choice.to_string(), s);
    }

    #[test]
    fn choice_with_show_only() {
        let s = "(visible)->prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.show, Some("visible".to_string()));
        assert_eq!(choice.enable, None);
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert!(choice.has_condition());
        assert_eq!(choice.to_string(), s);
    }

    #[test]
    fn choice_with_enable_only() {
        let s = "[enabled]->prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.show, None);
        assert_eq!(choice.enable, Some("enabled".to_string()));
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert!(choice.has_condition());
        assert_eq!(choice.to_string(), s);
    }

    #[test]
    fn choice_with_both_conditions() {
        let s = "(show)[enable]->prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.show, Some("show".to_string()));
        assert_eq!(choice.enable, Some("enable".to_string()));
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert!(choice.has_condition());
        assert_eq!(choice.to_string(), s);

        let s_rev = "[enable](show)->prompt:target";
        let choice_rev = Choice::from_str(s_rev);
        assert_eq!(choice_rev.show, Some("show".to_string()));
        assert_eq!(choice_rev.enable, Some("enable".to_string()));
        assert_eq!(choice_rev.to_string(), "(show)[enable]->prompt:target");
    }

    #[test]
    fn choice_extra_content_in_condition() {
        let s = "xyz(show)abc[enable]def->prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.show, Some("show".to_string()));
        assert_eq!(choice.enable, Some("enable".to_string()));
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert_eq!(choice.to_string(), "(show)[enable]->prompt:target");
    }

    #[test]
    fn choice_escape_characters() {
        // 测试冒号转义
        let s = r"prompt\:with\:colon:target\:with\:colon";
        let choice = Choice::from_str(s);
        assert_eq!(choice.prompt, "prompt:with:colon");
        assert_eq!(choice.target, Some("target:with:colon".to_string()));
        assert_eq!(choice.to_string(), s);
    }

    #[test]
    fn choice_escape_in_condition() {
        // 条件中的转义 (反斜杠保持原样, 不处理 `\)` 或 `\]`) (这种不符合表达式语法所以不管)
        let s = r"(show\)with\)paren)[enable\]]->prompt:target";
        let choice = Choice::from_str(s);
        assert_eq!(choice.show, Some(r"show\".to_string()));
        assert_eq!(choice.enable, Some(r"enable\".to_string()));
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
        assert_eq!(choice.to_string(), r"(show\)[enable\]->prompt:target");
    }

    // -------- ChoiceView 直接测试 --------

    #[test]
    fn choice_view_basic() {
        let view = ChoiceView::from_str("prompt:target");
        assert_eq!(view.prompt, "prompt");
        assert_eq!(view.target, Some("target"));
        assert_eq!(view.show, None);
        assert_eq!(view.enable, None);
        assert!(!view.has_condition());
        assert_eq!(view.to_string(), "prompt:target");
        let choice: Choice = view.into();
        assert_eq!(choice.prompt, "prompt");
        assert_eq!(choice.target, Some("target".to_string()));
    }

    #[test]
    fn choice_view_with_condition() {
        let view = ChoiceView::from_str("(show)[enable]->prompt:target");
        assert_eq!(view.show, Some("show"));
        assert_eq!(view.enable, Some("enable"));
        assert_eq!(view.prompt, "prompt");
        assert_eq!(view.target, Some("target"));
        assert!(view.has_condition());
        assert_eq!(view.to_string(), "(show)[enable]->prompt:target");
    }

    // -------- ChoiceSplit 迭代器 --------

    #[test]
    fn choice_split_basic() {
        let s = "opt1:t1|opt2:t2|opt3:t3";
        let split = ChoiceSplit::new(s);
        let choices: Vec<Choice> = split.map(Choice::from).collect();
        assert_eq!(choices.len(), 3);
        assert_eq!(choices[0].prompt, "opt1");
        assert_eq!(choices[0].target, Some("t1".to_string()));
        assert_eq!(choices[1].prompt, "opt2");
        assert_eq!(choices[1].target, Some("t2".to_string()));
        assert_eq!(choices[2].prompt, "opt3");
        assert_eq!(choices[2].target, Some("t3".to_string()));
    }

    #[test]
    fn choice_split_with_conditions() {
        let s = "(show)[enable]->go:scene_a|(hide)[disabled]->stay:scene_b";
        let split = ChoiceSplit::new(s);
        let choices: Vec<Choice> = split.map(Choice::from).collect();
        assert_eq!(choices.len(), 2);
        let c1 = &choices[0];
        assert_eq!(c1.show, Some("show".to_string()));
        assert_eq!(c1.enable, Some("enable".to_string()));
        assert_eq!(c1.prompt, "go");
        assert_eq!(c1.target, Some("scene_a".to_string()));
        let c2 = &choices[1];
        assert_eq!(c2.show, Some("hide".to_string()));
        assert_eq!(c2.enable, Some("disabled".to_string()));
        assert_eq!(c2.prompt, "stay");
        assert_eq!(c2.target, Some("scene_b".to_string()));
    }

    #[test]
    fn choice_split_escape_pipe() {
        let s = r"prompt\|with\|pipe:target\|with\|pipe|another";
        let split = ChoiceSplit::new(s);
        let choices: Vec<Choice> = split.map(Choice::from).collect();
        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].prompt, "prompt|with|pipe");
        assert_eq!(choices[0].target, Some("target|with|pipe".to_string()));
        assert_eq!(choices[1].prompt, "another");
        assert_eq!(choices[1].target, None);
    }

    #[test]
    fn choice_split_empty() {
        let split = ChoiceSplit::new("");
        let choices: Vec<Choice> = split.map(Choice::from).collect();
        assert!(choices.is_empty());
    }

    #[test]
    fn choice_split_single_without_pipe() {
        let s = "only one";
        let split = ChoiceSplit::new(s);
        let choices: Vec<Choice> = split.map(Choice::from).collect();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].prompt, "only one");
        assert_eq!(choices[0].target, None);
    }

    // -------- Roundtrip 测试 --------

    #[test]
    fn choice_roundtrip() {
        let originals = vec![
            "simple",
            "prompt:target",
            "(show)->prompt:target",
            "[enable]->prompt:target",
            "(show)[enable]->prompt:target",
            r"escape\|colon\:prompt:target\:with\:colon",
            r"(show\)with\)paren)[enable\]]->prompt:target",
        ];
        for original in originals {
            let choice = Choice::from_str(original);
            let serialized = choice.to_string();
            let reparsed = Choice::from_str(&serialized);
            assert_eq!(choice, reparsed);
            assert_eq!(reparsed.to_string(), serialized);
        }
    }

    // -------- has_condition --------

    #[test]
    fn choice_has_condition() {
        let c1 = Choice::from_str("prompt");
        assert!(!c1.has_condition());
        let c2 = Choice::from_str("(show)->prompt");
        assert!(c2.has_condition());
        let c3 = Choice::from_str("[enable]->prompt");
        assert!(c3.has_condition());
        let c4 = Choice::from_str("(show)[enable]->prompt");
        assert!(c4.has_condition());
    }
}
