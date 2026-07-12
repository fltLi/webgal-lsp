use std::ops;

use lsp_types::*;
use rayon::prelude::*;
use webgal_model::{
    sentence::{Scene, Sentence, SentenceInfo},
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
                    token_types: TokenType::all().to_vec(),
                    token_modifiers: vec![],
                },
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
            static_registration_options: StaticRegistrationOptions::default(),
        },
    )
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
fn highlight_sentence<F>(sentence: &SentenceInfo, mut push: F)
where
    F: FnMut(PrimaryToken),
{
    let SentenceInfo {
        content,
        primary,
        sentence,
        ..
    } = sentence;

    // 语句类型高亮
    push(PrimaryToken {
        span: primary.get_span(primary.command),
        kind: if !sentence.is_say() {
            TokenType::Function
        } else if primary.content.is_some() {
            TokenType::Variable
        } else {
            TokenType::String
        },
    });

    // 主参数高亮
    if let Some(content) = primary.content {
        // `:`
        let pos = primary.command.len();
        push(PrimaryToken::from_position(pos, TokenType::Operator));

        if matches!(sentence, Sentence::Choose(_)) {
            // 参数值 (choose)
            highlight_choose_content(content, |mut token| {
                token.span.start += pos + 1;
                token.span.end += pos + 1;
                push(token)
            });
        } else if let Some(kind) = TokenType::from_content(sentence) {
            // 参数值 (常规)
            push(PrimaryToken {
                span: primary.get_span(content),
                kind,
            });
        }
    }

    // 参数高亮
    for (name, value) in primary.arguments.iter() {
        let span = primary.get_span(name);
        let ops::Range { start, end } = span;

        // `-`
        push(PrimaryToken::from_position(start - 1, TokenType::Operator));

        // 参数名
        push(PrimaryToken {
            span,
            kind: TokenType::Property,
        });

        // `=`
        if value.is_some() {
            push(PrimaryToken::from_position(end, TokenType::Operator));
        }

        // 参数值
        if let Some(value) = value
            && let Some(kind) = TokenType::from_arguemnt(name, sentence)
        {
            push(PrimaryToken {
                span: primary.get_span(value),
                kind,
            });
        }
    }

    // 注释高亮
    let comment = content
        .len()
        .checked_sub(primary.comment.len() + 1)
        .and_then(|pos| content.get(pos..))
        .filter(|comment| comment.starts_with(';'))
        .unwrap_or(primary.comment);
    if !comment.is_empty() {
        push(PrimaryToken {
            span: primary.get_span(comment),
            kind: TokenType::Comment,
        });
    }
}

fn highlight_choose_content<F>(content: &str, mut push: F)
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
                push(PrimaryToken {
                    span,
                    kind: TokenType::Regex,
                });

                // `->`
                push(PrimaryToken {
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
            push(PrimaryToken {
                span: span_of(content, prompt),
                kind: TokenType::String,
            });
        }

        if let Some(target) = target {
            let span = span_of(content, target);
            let start = span.start;

            // `:`
            push(PrimaryToken::from_position(start - 1, TokenType::Operator));

            // 场景 / 标签
            push(PrimaryToken {
                span,
                kind: TokenType::Regex,
            });
        }

        if with_trailing_delimiter {
            // `|`
            push(PrimaryToken::from_position(
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
    Variable,
    Property,
    EnumMember,
    Function,
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

    fn from_arguemnt(name: &str, sentence: &Sentence) -> Option<Self> {
        match name {
            // 标识符
            "speaker" | "figureId" | "id" | "target" => Some(Self::Variable),
            "name" | "unlockname" | "series" => Some(Self::Variable),
            "achivementId" => Some(Self::Variable),

            // 枚举
            "fontSize" => Some(Self::EnumMember),
            "exit" | "ease" | "animation" => Some(Self::EnumMember),
            "enter" if !matches!(sentence, Sentence::Bgm(_)) => Some(Self::EnumMember),

            // 文本 / JSON / ...
            "title" | "buttonText" | "ruleText" | "ruleButtonText" => Some(Self::String),
            "transform" => Some(Self::String),
            "bounds" | "blink" | "focus" => Some(Self::String),
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
            Self::Variable => 0,
            Self::Property => 1,
            Self::EnumMember => 2,
            Self::Function => 3,
            Self::Comment => 4,
            Self::String => 5,
            Self::Number => 6,
            Self::Regex => 7,
            Self::Operator => 8,
        }
    }

    const fn all() -> &'static [SemanticTokenType] {
        const TOKEN_TYPES: &[SemanticTokenType] = &[
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::FUNCTION,
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
            TokenType::Variable => Self::VARIABLE,
            TokenType::Property => Self::PROPERTY,
            TokenType::EnumMember => Self::ENUM_MEMBER,
            TokenType::Function => Self::FUNCTION,
            TokenType::Comment => Self::COMMENT,
            TokenType::String => Self::STRING,
            TokenType::Number => Self::NUMBER,
            TokenType::Regex => Self::REGEXP,
            TokenType::Operator => Self::OPERATOR,
        }
    }
}
