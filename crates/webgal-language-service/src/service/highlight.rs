use std::ops;

use json_language_service::TokenType as JsonTokenType;
use lsp_types::*;
use rayon::prelude::*;
use webgal_language_core::{
    element::{InterpolateItem, InterpolateSplit, TokenSplit},
    sentence::{
        PrimarySentence, Scene, Sentence, SentenceInfo, is_call_scene_variable_argument,
        is_implicit_vocal_argument,
    },
    util::{span_of, split_once_escaped},
};

pub fn highlight_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
        SemanticTokensRegistrationOptions {
            text_document_registration_options: TextDocumentRegistrationOptions {
                document_selector: Some(vec![DocumentFilter {
                    language: Some("webgal".to_string()),
                    scheme: Some("file".to_string()),
                    pattern: Some("**/scene/**/*.txt".to_string()),
                }]),
            },
            semantic_tokens_options: SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: token_types().to_vec(),
                    token_modifiers: vec![],
                },
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
            static_registration_options: StaticRegistrationOptions::default(),
        },
    )
}

pub const fn token_types() -> &'static [SemanticTokenType] {
    TokenType::all()
}

/// 为场景提供语义高亮
pub fn highlight(scene: &Scene) -> Vec<SemanticToken> {
    let span = Range {
        start: Position::default(),
        end: Position {
            line: scene.sentences().len().saturating_sub(1) as u32,
            character: scene
                .sentences()
                .last()
                .map(|sentence| sentence.content.len())
                .unwrap_or(0) as u32,
        },
    };
    highlight_range(scene, span)
}

/// 为局部场景提供高亮
///
/// `span` 为 UTF-8 行列. 行号增量为**文档绝对行号** (相对第 0 行), 因此结果可直接
/// 交给 [`crate::encode::highlights_utf8_to_utf16`] 编码为 LSP `SemanticTokens`.
///
/// 区间按行对齐: 起点行按 `span.start.character` 裁剪, 末行不裁剪 (整行返回).
pub fn highlight_range(scene: &Scene, span: Range) -> Vec<SemanticToken> {
    // 并行处理每条语句
    let mut lines: Vec<_> = scene
        .sentences()
        .get(span.start.line as usize..=span.end.line as usize)
        .unwrap_or_default()
        .par_iter()
        .enumerate()
        .filter_map(|(line, sentence)| {
            let line = line + span.start.line as usize;
            let mut tokens: Vec<SemanticToken> = Vec::new();
            let mut last_end = 0;

            highlight_sentence(
                sentence,
                |PrimaryToken {
                     span: ops::Range { start, end },
                     kind,
                 }| {
                    let delta_start = (start - last_end) as u32;
                    let length = (end - start) as u32;
                    last_end = end;

                    tokens.push(SemanticToken {
                        delta_line: 0,
                        delta_start,
                        length,
                        token_type: kind.to_id(),
                        token_modifiers_bitset: 0,
                    });
                },
            );

            if tokens.is_empty() {
                None
            } else {
                Some((line, tokens))
            }
        })
        .collect();

    // 对齐首行
    if let Some((_, tokens)) = lines
        .first_mut()
        .filter(|(line, _)| *line == span.start.line as usize)
    {
        let mut previous_end = 0;
        let mut first_token = 0;

        for (index, token) in tokens.iter_mut().enumerate() {
            let token_start = previous_end + token.delta_start;
            let token_end = token_start + token.length;
            previous_end = token_end;

            if token_end <= span.start.character {
                first_token = index + 1;
                continue;
            }

            if token_start < span.start.character {
                token.length = token_end - span.start.character;
                token.delta_start = span.start.character;
            } else {
                token.delta_start = token_start;
            }
            break;
        }

        tokens.drain(..first_token);
    }

    // 追加行递增 (行号增量为文档绝对行号)
    let mut last_line = 0;
    lines
        .into_iter()
        .filter(|(_, tokens)| !tokens.is_empty())
        .flat_map(|(line, mut tokens)| {
            tokens[0].delta_line = (line - last_line) as u32;
            last_line = line;
            tokens
        })
        .collect()
}

/// 生成一条语句的高亮
fn highlight_sentence<F>(sentence: &SentenceInfo, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let SentenceInfo {
        content,
        primary,
        sentence,
        ..
    } = sentence;

    highlight_command(primary, sentence, &mut f);
    highlight_content(primary, sentence, &mut f);

    // 参数高亮
    for &(name, value) in primary.arguments.iter() {
        highlight_argument(name, value, primary, sentence, &mut f);
    }

    highlight_comment(content, primary.comment, &mut f);
}

/// 语句类型高亮
fn highlight_command<F>(primary: &PrimarySentence, sentence: &Sentence, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    if !sentence.is_say() {
        f(PrimaryToken {
            span: primary.get_span(primary.command),
            kind: TokenType::Function,
        });
    } else if primary.content.is_some() {
        // 对话者
        highlight_interpolate(
            primary.command,
            TokenType::Type,
            |s| primary.get_span(s),
            &mut f,
        );
    } else {
        // 对话内容
        highlight_say_content(primary.command, &mut f);
    }
}

/// 语句主参数高亮
fn highlight_content<F>(primary: &PrimarySentence, sentence: &Sentence, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    if let Some(content) = primary.content {
        // `:`
        let span = primary.get_span(content);
        f(PrimaryToken::from_position(
            primary.command.len(),
            TokenType::Operator,
        ));

        let shifted_push = |mut token: PrimaryToken| {
            token.span.start += span.start;
            token.span.end += span.start;
            f(token)
        };

        // 参数值
        match sentence {
            Sentence::Say(_) => highlight_say_content(content, shifted_push),
            Sentence::SetTransform(_) | Sentence::SetTempAnimation(_) => {
                highlight_json(content, shifted_push)
            }
            Sentence::Intro(_) => highlight_intro_content(content, shifted_push),
            Sentence::Choose(_) => highlight_choose_content(content, shifted_push),
            Sentence::SetVariable(_) => highlight_set_variable_content(content, shifted_push),
            Sentence::ApplyStyle(_) => highlight_apply_style_content(content, shifted_push),
            _ if let Some(kind) = TokenType::from_content(sentence) => f(PrimaryToken {
                span: primary.get_span(content),
                kind,
            }),
            _ => {}
        }
    }
}

/// 语句参数高亮
fn highlight_argument<F>(
    name: &str,
    value: Option<&str>,
    primary: &PrimarySentence,
    sentence: &Sentence,
    mut f: F,
) where
    F: FnMut(PrimaryToken),
{
    let span = primary.get_span(name);
    let ops::Range { start, end } = span;
    let is_variable = matches!(sentence, Sentence::CallScene(_))
        && value.is_some()
        && is_call_scene_variable_argument(name);

    // `-`
    f(PrimaryToken::from_position(start - 1, TokenType::Operator));

    // 参数名
    if sentence.is_say() && value.is_none() && is_implicit_vocal_argument(name) {
        f(PrimaryToken {
            span,
            kind: TokenType::Regex,
        });
    } else if is_variable {
        f(PrimaryToken {
            span,
            kind: TokenType::Variable,
        });
    } else {
        f(PrimaryToken {
            span,
            kind: TokenType::Parameter,
        });
    }

    // `=`
    if value.is_some() {
        f(PrimaryToken::from_position(end, TokenType::Operator));
    }

    // 参数值
    if let Some(value) = value {
        let span = primary.get_span(value);
        if is_variable {
            f(PrimaryToken {
                span,
                kind: TokenType::Regex,
            });
        } else if sentence.is_say() && matches!(name, "speaker") {
            highlight_interpolate(value, TokenType::Type, |s| primary.get_span(s), f);
        } else if matches!(name, "transform" | "bounds" | "blink" | "focus") {
            highlight_json(value, |mut token| {
                token.span.start += span.start;
                token.span.end += span.start;
                f(token)
            });
        } else if let Some(kind) = TokenType::from_argument(name, sentence) {
            f(PrimaryToken { span, kind });
        }
    }
}

/// 语句注释高亮
fn highlight_comment<F>(content: &str, comment: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let ops::Range { start, end } = span_of(content, comment);

    if let Some(position) = start.checked_sub(1)
        && let Some(";") = content.get(position..position + 1)
    {
        // `;`
        f(PrimaryToken::from_position(position, TokenType::Comment));
    }

    // 普通注释
    if !comment.starts_with("nolint:") {
        // 注释
        f(PrimaryToken {
            span: start..end,
            kind: TokenType::Comment,
        });

        return;
    }

    // `nolint`
    f(PrimaryToken {
        span: start..start + 6,
        kind: TokenType::Keyword,
    });

    // `:`
    f(PrimaryToken::from_position(start + 6, TokenType::Operator));

    let mut last_end = start + 7;
    for (i, ch) in comment[7..].char_indices() {
        let i = start + 7 + i;

        match ch {
            '|' => {
                // 诊断码
                f(PrimaryToken {
                    span: last_end..i,
                    kind: TokenType::EnumMember,
                });

                // `|`
                f(PrimaryToken::from_position(i, TokenType::Operator));

                last_end = i + 1;
            }

            ';' => {
                // 诊断码
                f(PrimaryToken {
                    span: last_end..i,
                    kind: TokenType::EnumMember,
                });

                // 注释
                f(PrimaryToken {
                    span: i..end,
                    kind: TokenType::Comment,
                });

                last_end = end;
                break;
            }

            _ => {}
        }
    }

    if last_end != end {
        // 诊断码
        f(PrimaryToken {
            span: last_end..end,
            kind: TokenType::EnumMember,
        });
    }
}

fn highlight_say_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let mut text_split = content.split('|').peekable();
    while let Some(text) = text_split.next() {
        for token in TokenSplit::new(text) {
            // 文本
            if !token.text.is_empty() {
                highlight_interpolate(
                    token.text,
                    TokenType::String,
                    |s| span_of(content, s),
                    &mut f,
                );
            }

            // 注音和样式
            if let Some(style) = token.get_full_style() {
                f(PrimaryToken {
                    span: span_of(content, style),
                    kind: TokenType::Regex,
                })
            }
        }

        if text_split.peek().is_some() {
            // `|`
            f(PrimaryToken::from_position(
                span_of(content, text).end,
                TokenType::Operator,
            ));
        }
    }
}

fn highlight_intro_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let mut text_split = content.split('|').peekable();
    while let Some(text) = text_split.next() {
        // 文本
        if !text.is_empty() {
            highlight_interpolate(text, TokenType::String, |s| span_of(content, s), &mut f);
        }

        if text_split.peek().is_some() {
            // `|`
            f(PrimaryToken::from_position(
                span_of(content, text).end,
                TokenType::Operator,
            ));
        }
    }
}

fn highlight_choose_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    // 单个选项解析
    let mut highlight_choice = |choice: &str, with_trailing_delimiter| {
        let body = match choice.split_once("->") {
            Some((condition, body)) => {
                let span = span_of(content, condition);
                let end = span.end;

                // 条件表达式
                f(PrimaryToken {
                    span,
                    kind: TokenType::Regex,
                });

                // `->`
                f(PrimaryToken {
                    span: end..end + 2,
                    kind: TokenType::Operator,
                });

                body
            }
            None => choice,
        };

        let (prompt, target) = match split_once_escaped(body, ':') {
            Some((prompt, target)) => (prompt, Some(target)),
            None => (body, None),
        };

        if !prompt.is_empty() {
            // 显示文本
            highlight_interpolate(prompt, TokenType::String, |s| span_of(content, s), &mut f);
        }

        if let Some(target) = target {
            let span = span_of(content, target);
            let start = span.start;

            // `:`
            f(PrimaryToken::from_position(start - 1, TokenType::Operator));

            // 场景 / 标签
            f(PrimaryToken {
                span,
                kind: TokenType::Regex,
            });
        }

        if with_trailing_delimiter {
            // `|`
            f(PrimaryToken::from_position(
                span_of(content, choice).end,
                TokenType::Operator,
            ));
        }
    };

    // 循环解析选项
    let mut text = content;
    while !text.is_empty() {
        match split_once_escaped(text, '|') {
            Some((choice, remain)) => {
                highlight_choice(choice, true);
                text = remain;
            }
            None => {
                // 最后一个选项
                highlight_choice(text, false);
                break;
            }
        }
    }
}

fn highlight_set_variable_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let (variable, expression) = match content.split_once('=') {
        Some(v) => v,
        None => return,
    };

    // 变量
    f(PrimaryToken {
        span: span_of(content, variable),
        kind: TokenType::Variable,
    });

    // `=`
    f(PrimaryToken::from_position(
        variable.len(),
        TokenType::Operator,
    ));

    // 表达式
    f(PrimaryToken {
        span: span_of(content, expression),
        kind: TokenType::Regex,
    });
}

fn highlight_apply_style_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    for (previous, current) in content
        .split(',')
        .flat_map(|change| change.split_once("->"))
    {
        // 原样式
        let previous_span = span_of(content, previous);
        f(PrimaryToken {
            span: previous_span.clone(),
            kind: TokenType::Regex,
        });

        // `->`
        f(PrimaryToken {
            span: previous_span.end..previous_span.end + 2,
            kind: TokenType::Operator,
        });

        // 新样式
        f(PrimaryToken {
            span: span_of(content, current),
            kind: TokenType::Regex,
        });
    }
}

fn highlight_interpolate<F, S>(s: &str, kind: TokenType, mut span_of: S, mut f: F)
where
    F: FnMut(PrimaryToken),
    S: FnMut(&str) -> ops::Range<usize>,
{
    for item in InterpolateSplit::new(s) {
        match item {
            InterpolateItem::Text(text) => {
                // 纯文本
                f(PrimaryToken {
                    span: span_of(text),
                    kind,
                });
            }

            InterpolateItem::Variable(name) => {
                // 变量插值
                let span = span_of(name);
                f(PrimaryToken::from_position(
                    span.start - 1,
                    TokenType::Operator,
                ));
                f(PrimaryToken {
                    span: span.clone(),
                    kind: TokenType::Variable,
                });
                f(PrimaryToken::from_position(span.end, TokenType::Operator));
            }
        }
    }
}

fn highlight_json<F>(s: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    json_language_service::highlight(s, |span, kind| {
        f(PrimaryToken {
            span,
            kind: kind.into(),
        })
    });
}

struct PrimaryToken {
    span: ops::Range<usize>,
    kind: TokenType,
}

impl PrimaryToken {
    fn from_position(position: usize, kind: TokenType) -> Self {
        Self {
            span: position..position + 1,
            kind,
        }
    }
}

#[derive(Clone, Copy)]
enum TokenType {
    Type,
    Parameter,
    Variable,
    #[allow(dead_code)]
    Property,
    EnumMember,
    Function,
    Keyword,
    Comment,
    String,
    Number,
    Regex, // 也表示路径
    Operator,
}

impl TokenType {
    fn from_content(sentence: &Sentence) -> Option<Self> {
        macro_rules! from_content_match {
            ($sentence:ident: {$($variant:ident => $kind:ident),* $(,)?}) => {{
                match $sentence {
                    $(Sentence::$variant(_) => Some(Self::$kind),)*
                    _ => None,
                }
            }};
        }

        from_content_match! {
            sentence: {
                // 常规演出
                // Say => String, // 已由调用者接管
                ChangeBackground => Regex,
                ChangeFigure => Regex,
                Bgm => Regex,
                PlayVideo => Regex,
                PlayEffect => Regex,

                // 舞台对象控制
                SetAnimation => EnumMember,
                SetComplexAnimation => EnumMember,
                // SetTransform => String, // 已由调用者接管
                // SetTempAnimation => String, // 已由调用者接管

                // 特殊演出
                PixiPerform => EnumMember,
                // Intro => String, // 已由调用者接管
                MiniAvatar => Regex,
                SetTextbox => EnumMember,
                FilmMode => EnumMember,

                // 场景与分支
                CallScene => Regex,
                ChangeScene => Regex,
                // Choose => String, // 已由调用者接管
                Label => Variable,
                JumpLabel => Variable,

                // 鉴赏
                UnlockCg => Regex,
                UnlockBgm => Regex,

                // 游戏控制
                GetUserInput => Variable,
                // SetVariable => Regex, // 已由调用者接管
                Wait => Number,
                // ApplyStyle => Regex, // 已由调用者接管
            }
        }
    }

    fn from_argument(name: &str, sentence: &Sentence) -> Option<Self> {
        match name {
            // 标识符
            "speaker" => Some(Self::Type),
            "figureId" | "id" | "target" => Some(Self::Variable),
            "name" | "unlockname" | "series" => Some(Self::Variable),
            "achivementId" => Some(Self::Variable),

            // 枚举
            "fontSize" => Some(Self::EnumMember),
            "exit" | "ease" | "animation" => Some(Self::EnumMember),
            "enter" if !matches!(sentence, Sentence::Bgm(_)) => Some(Self::EnumMember),

            // 文本 / ...
            "title" | "buttonText" | "ruleText" | "ruleButtonText" => Some(Self::String),
            // "transform" => Some(Self::String), // 已由调用者接管
            // "bounds" | "blink" | "focus" => Some(Self::String), // 已由调用者接管
            "fontColor" | "backgroundColor" => Some(Self::String),

            // 时间 / 序号 / ...
            "duration" | "enterDuration" | "exitDuration" | "delayTime" => Some(Self::Number),
            "volume" | "enter" => Some(Self::Number),
            "zIndex" => Some(Self::Number),
            "defaultChoice" => Some(Self::Number),

            // 路径
            "vocal" => Some(Self::Regex),
            "backgroundImage" => Some(Self::Regex),
            "mouthOpen" | "mouthHalfOpen" | "mouthClose" | "eyesOpen" | "eyesClose" => {
                Some(Self::Regex)
            }
            "skin" | "motion" | "expression" => Some(Self::Regex),

            // 表达式
            "defaultValue" | "rule" | "ruleFlag" => Some(Self::Regex),
            "when" => Some(Self::Regex),

            // 变量
            "writeReturnTo" => Some(Self::Parameter),

            _ => None,
        }
    }

    fn to_id(self) -> u32 {
        match self {
            Self::Type => 0,
            Self::Variable => 1,
            Self::Parameter => 2,
            Self::Property => 3,
            Self::EnumMember => 4,
            Self::Function => 5,
            Self::Keyword => 6,
            Self::Comment => 7,
            Self::String => 8,
            Self::Number => 9,
            Self::Regex => 10,
            Self::Operator => 11,
        }
    }

    const fn all() -> &'static [SemanticTokenType] {
        const TOKEN_TYPES: &[SemanticTokenType] = &[
            SemanticTokenType::TYPE,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
        ];
        TOKEN_TYPES
    }
}

impl From<TokenType> for SemanticTokenType {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::Type => Self::TYPE,
            TokenType::Variable => Self::VARIABLE,
            TokenType::Parameter => Self::PARAMETER,
            TokenType::Property => Self::PROPERTY,
            TokenType::EnumMember => Self::ENUM_MEMBER,
            TokenType::Function => Self::FUNCTION,
            TokenType::Keyword => Self::KEYWORD,
            TokenType::Comment => Self::COMMENT,
            TokenType::String => Self::STRING,
            TokenType::Number => Self::NUMBER,
            TokenType::Regex => Self::REGEXP,
            TokenType::Operator => Self::OPERATOR,
        }
    }
}

impl From<JsonTokenType> for TokenType {
    fn from(value: JsonTokenType) -> Self {
        match value {
            JsonTokenType::Keyword => Self::Keyword,
            JsonTokenType::String => Self::String,
            JsonTokenType::Number => Self::Number,
            JsonTokenType::Operator => Self::Operator,
        }
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use crate::encode::{highlights_utf8_to_utf16, range_utf16_to_utf8};

    use super::*;

    fn token_ranges(tokens: &[SemanticToken]) -> Vec<(u32, u32, u32)> {
        let mut line = 0;
        let mut previous_end = 0;
        tokens
            .iter()
            .map(|token| {
                if token.delta_line > 0 {
                    line += token.delta_line;
                    previous_end = 0;
                }
                let start = previous_end + token.delta_start;
                let end = start + token.length;
                previous_end = end;
                (line, start, end)
            })
            .collect()
    }

    #[test]
    fn highlight_range_clips_start_line_only() {
        let scene = Scene::from_str("wait:123;\nwait:456;");
        let tokens = highlight_range(
            &scene,
            Range {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 1,
                    character: 7,
                },
            },
        );

        let ranges = token_ranges(&tokens);
        // 起点行按 span.start.character 裁剪
        assert!(
            ranges
                .iter()
                .any(|(line, start, _)| *line == 0 && *start == 2)
        );
        assert!(
            ranges
                .iter()
                .all(|(line, start, _)| *line != 0 || *start >= 2)
        );
        // 末行不裁剪: span.end.character 之后的高亮也一并返回
        assert!(ranges.iter().any(|(line, _, end)| *line == 1 && *end == 9));
    }

    #[test]
    fn highlight_range_clips_start_column_on_single_line() {
        let scene = Scene::from_str("wait:123;");
        let tokens = highlight_range(
            &scene,
            Range {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 7,
                },
            },
        );

        // (行, 起, 止): 首令牌裁到第 2 列; 第 7 列之后的 `;` 与空注释不裁剪, 仍然返回
        assert_eq!(
            token_ranges(&tokens),
            vec![(0, 2, 4), (0, 4, 5), (0, 5, 8), (0, 8, 9), (0, 9, 9)]
        );
    }

    #[test]
    fn highlight_range_from_middle_line() {
        let scene = Scene::from_str("wait:123;\nwait:456;\nwait:789;");
        let tokens = highlight_range(
            &scene,
            Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 5,
                },
            },
        );

        // 行号增量为文档绝对行号 (相对第 0 行), 而非相对区间起点或切片下标
        assert_eq!(tokens[0].delta_line, 1);
        let ranges = token_ranges(&tokens);
        assert!(ranges.iter().any(|(line, _, _)| *line == 1));
        // 末行不按 span.end.character 截断: 整行高亮都在
        assert!(ranges.iter().any(|(line, _, end)| *line == 2 && *end == 9));
    }

    #[test]
    fn highlight_range_drops_line_emptied_by_clipping() {
        let scene = Scene::from_str("wait:123;\nwait:456;");
        let tokens = highlight_range(
            &scene,
            Range {
                start: Position {
                    line: 0,
                    character: 9,
                },
                end: Position {
                    line: 1,
                    character: 9,
                },
            },
        );

        // 首行 token 全部落在区间起点之前: 该行应被丢弃, 而不是留下空 token 列表
        assert_eq!(tokens[0].delta_line, 1);
        assert!(token_ranges(&tokens).iter().all(|(line, _, _)| *line == 1));
    }

    #[test]
    fn highlight_range_takes_utf8_columns() {
        // 入参为 UTF-8 偏移: 调用方需先经 encode::range_utf16_to_utf8 转换
        let scene = Scene::from_str("中文对话:内容;");
        let tokens = highlight_range(
            &scene,
            range_utf16_to_utf8(
                &scene,
                Range {
                    start: Position {
                        line: 0,
                        character: 1,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
            ),
        );

        // UTF-16 第 1 列 = 字节 3: 首令牌起点按字节裁剪; 末列不裁剪, 整行高亮都返回
        assert_eq!(
            token_ranges(&tokens),
            vec![
                (0, 3, 12),
                (0, 12, 13),
                (0, 13, 19),
                (0, 19, 20),
                (0, 20, 20)
            ]
        );
    }

    #[test]
    fn highlight_range_encodes_absolute_lines() {
        // 区间高亮按 LSP `SemanticTokens.data` 发送: 首个令牌的 delta_line 必须是文档绝对行号
        // (相对第 0 行), 否则 Monaco 会把该区间的高亮画到文档开头 —— toMultilineTokens2 从第 1 行
        // 开始累加 delta_line, 而 setPartialSemanticTokens 只按解码出的文档行号落位。
        let scene = Scene::from_str("中文对话:内容;\nwait:456;");
        let span = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 9,
            },
        };
        let mut tokens = highlight_range(&scene, range_utf16_to_utf8(&scene, span));
        highlights_utf8_to_utf16(&scene, &mut tokens);

        let flat: Vec<_> = tokens
            .iter()
            .map(|token| (token.delta_line, token.delta_start, token.length))
            .collect();
        // (delta_line, delta_start, length); delta_start 相对上一个令牌起点,
        // 解码后即第 1 行的 wait 0..4 / ':' 4..5 / '456' 5..8 / ';' 8..9 / 空注释 9..9
        assert_eq!(
            flat,
            vec![(1, 0, 4), (0, 4, 1), (0, 1, 3), (0, 3, 1), (0, 1, 0)]
        );
    }

    #[test]
    fn highlight_choose_various_cases() {
        let test_cases = vec![
            // 基础: 多个选项
            "choose:opt1:scene_a|opt2:scene_b|opt3:scene_c;",
            // 带条件
            "choose:(show)[enable]->go:scene_a|(hide)[disabled]->stay:scene_b;",
            // 只有一个选项
            "choose:only;",
            // 选项以 `->` 结尾 (无 target)
            "choose:opt1->;",
            // 选项以 `|` 结尾 (空选项)
            "choose:opt1:target|;",
            // 选项内容含转义
            r"choose:prompt\|with\|pipe:target\|with\|pipe;",
            // 条件为空 (可能实际语法不支持, 但测试边界)
            "choose:():->go;",
            // 混合情况
            "choose:(cond)->opt1|opt2:target;",
            // 空选项集合 (仅 `choose:;`)
            "choose:;",
            // 多个 `|` 结尾
            "choose:opt1:target||;",
            // `target` 为空 (仅 `prompt:`)
            "choose:prompt:|;",
            // `prompt` 为空 (仅 `:target`)
            "choose::target;",
        ];

        for (i, case) in test_cases.iter().enumerate() {
            let scene = Scene::from_str(*case);
            let tokens = highlight(&scene);
            // 至少有一个 token (语句本身), 且无 panic
            assert!(!tokens.is_empty(), "Test case {i}: no tokens");
        }
    }

    #[test]
    fn highlight_main_content_after_leading_spaces() {
        let sentence = SentenceInfo::from_str("choose:  prompt:target;");
        let mut tokens = Vec::new();

        highlight_sentence(&sentence, |token| {
            tokens.push((token.span, token.kind.to_id()))
        });

        assert_eq!(
            tokens,
            vec![
                (0..6, TokenType::Function.to_id()),
                (6..7, TokenType::Operator.to_id()),
                (9..15, TokenType::String.to_id()),
                (15..16, TokenType::Operator.to_id()),
                (16..22, TokenType::Regex.to_id()),
                (22..23, TokenType::Comment.to_id()),
                (23..23, TokenType::Comment.to_id()),
            ]
        );
    }

    #[test]
    fn highlight_variable_interpolation_in_say_content() {
        let sentence = SentenceInfo::from_str("Alice:Hello {name}!;");
        let mut tokens = Vec::new();

        highlight_sentence(&sentence, |token| {
            tokens.push((token.span, token.kind.to_id()))
        });

        assert_eq!(
            tokens,
            vec![
                (0..5, TokenType::Type.to_id()),
                (5..6, TokenType::Operator.to_id()),
                (6..12, TokenType::String.to_id()),
                (12..13, TokenType::Operator.to_id()),
                (13..17, TokenType::Variable.to_id()),
                (17..18, TokenType::Operator.to_id()),
                (18..19, TokenType::String.to_id()),
                (19..20, TokenType::Comment.to_id()),
                (20..20, TokenType::Comment.to_id()),
            ]
        );
    }
}
