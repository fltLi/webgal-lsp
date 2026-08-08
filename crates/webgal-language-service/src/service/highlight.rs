use std::ops;

use json_language_service::TokenType as JsonTokenType;
use lsp_types::*;
use rayon::prelude::*;
use webgal_language_core::{
    element::TokenSplit,
    sentence::{PrimarySentence, Scene, Sentence, SentenceInfo},
    util::{span_of, split_once_escaped},
};

// TODO: 变量插值高亮

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
                range: Some(false),
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
    // 并行处理每条语句
    let lines: Vec<_> = scene
        .sentences()
        .par_iter()
        .enumerate()
        .filter_map(|(line, sentence)| {
            let mut tokens = Vec::new();
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

    // 追加行递增
    let mut last_line = 0;
    lines
        .into_iter()
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

    highlight_comment(content, primary, &mut f);
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
        f(PrimaryToken {
            span: primary.get_span(primary.command),
            kind: TokenType::Type,
        });
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
        let pos = primary.command.len();
        f(PrimaryToken::from_position(pos, TokenType::Operator));

        let shifted_push = |mut token: PrimaryToken| {
            token.span.start += pos + 1;
            token.span.end += pos + 1;
            f(token)
        };

        // 参数值
        match sentence {
            Sentence::Say(_) => highlight_say_content(content, shifted_push),
            Sentence::Choose(_) => highlight_choose_content(content, shifted_push),
            Sentence::SetTransform(_) | Sentence::SetTempAnimation(_) => {
                highlight_json(content, shifted_push)
            }
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

    // `-`
    f(PrimaryToken::from_position(start - 1, TokenType::Operator));

    // 参数名
    f(PrimaryToken {
        span,
        kind: TokenType::Parameter,
    });

    // `=`
    if value.is_some() {
        f(PrimaryToken::from_position(end, TokenType::Operator));
    }

    // 参数值
    if let Some(value) = value {
        if matches!(name, "transform" | "bounds" | "blink" | "focus") {
            let start = primary.get_span(value).start;
            highlight_json(value, |mut token| {
                token.span.start += start;
                token.span.end += start;
                f(token)
            });
        } else if let Some(kind) = TokenType::from_argument(name, sentence) {
            f(PrimaryToken {
                span: primary.get_span(value),
                kind,
            });
        }
    }
}

/// 语句注释高亮
fn highlight_comment<F>(content: &str, primary: &PrimarySentence, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    let comment = content
        .len()
        .checked_sub(primary.comment.len() + 1)
        .and_then(|pos| content.get(pos..))
        .filter(|comment| comment.starts_with(';'))
        .unwrap_or(primary.comment);
    if !comment.is_empty() {
        f(PrimaryToken {
            span: primary.get_span(comment),
            kind: TokenType::Comment,
        });
    }
}

fn highlight_say_content<F>(content: &str, mut f: F)
where
    F: FnMut(PrimaryToken),
{
    for token in content.split('|').flat_map(TokenSplit::new) {
        // 文本
        if !token.text.is_empty() {
            f(PrimaryToken {
                span: span_of(content, token.text),
                kind: TokenType::String,
            });
        }

        // 注音和样式
        if let Some(style) = token.get_full_style() {
            f(PrimaryToken {
                span: span_of(content, style),
                kind: TokenType::Regex,
            })
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
            f(PrimaryToken {
                span: span_of(content, prompt),
                kind: TokenType::String,
            });
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
                Say => String,
                ChangeBackground => Regex,
                ChangeFigure => Regex,
                Bgm => Regex,
                PlayVideo => Regex,
                PlayEffect => Regex,

                // 舞台对象控制
                SetAnimation => EnumMember,
                SetComplexAnimation => EnumMember,
                SetTransform => String,
                SetTempAnimation => String,

                // 特殊演出
                PixiPerform => EnumMember,
                Intro => String,
                MiniAvatar => Regex,
                SetTextbox => EnumMember,
                FilmMode => EnumMember,

                // 场景与分支
                CallScene => Regex,
                ChangeScene => Regex,
                Choose => String, // 已由调用者接管
                Label => Variable,
                JumpLabel => Variable,

                // 鉴赏
                UnlockCg => Regex,
                UnlockBgm => Regex,

                // 游戏控制
                GetUserInput => Variable,
                SetVar => Regex,
                Wait => Number,
                ApplyStyle => Regex,
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
        }
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

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
}
