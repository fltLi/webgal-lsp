use std::ops;

use tower_lsp::lsp_types::*;
use webgal_model::sentence::{Scene, Sentence, SentenceInfo};

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
    let mut tokens = Vec::new();
    let mut last_line = 0;

    for (line, sentence) in scene.sentences().iter().enumerate() {
        let mut last_end = 0;

        highlight_sentence(sentence, |PrimaryToken { start, end, kind }| {
            let delta_line = if line == last_line {
                0
            } else {
                let delta = (line - last_line) as u32;
                last_line = line;
                delta
            };

            let delta_start = (start - last_end) as u32;
            let length = (end - start) as u32;
            last_end = end;

            tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: kind.to_id(),
                token_modifiers_bitset: 0,
            });
        });
    }

    tokens
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
    push(PrimaryToken::from_span(
        primary.get_span(primary.command),
        if sentence.is_say() {
            TokenType::Variable
        } else {
            TokenType::Function
        },
    ));

    // 主参数高亮
    if let Some(content) = primary.content {
        // `:`
        let pos = primary.command.len();
        push(PrimaryToken {
            start: pos,
            end: pos + 1,
            kind: TokenType::Operator,
        });

        // 参数值
        if let Some(kind) = TokenType::from_content(sentence) {
            push(PrimaryToken::from_span(primary.get_span(content), kind));
        }
    }

    // 参数高亮
    for (name, value) in primary.arguments.iter() {
        let span = primary.get_span(name);
        let ops::Range { start, end } = span;

        // `-`
        push(PrimaryToken {
            start: start - 1,
            end: start,
            kind: TokenType::Operator,
        });

        // 参数名
        push(PrimaryToken::from_span(span, TokenType::Property));

        // `=`
        if value.is_some() {
            push(PrimaryToken {
                start: end,
                end: end + 1,
                kind: TokenType::Operator,
            });
        }

        // 参数值
        if let Some(value) = value
            && let Some(kind) = TokenType::from_arguemnt(name, sentence)
        {
            push(PrimaryToken::from_span(primary.get_span(value), kind));
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
        push(PrimaryToken::from_span(
            primary.get_span(comment),
            TokenType::Comment,
        ));
    }
}

#[derive(Clone, Copy)]
struct PrimaryToken {
    start: usize,
    end: usize,
    kind: TokenType,
}

impl PrimaryToken {
    fn from_span(span: ops::Range<usize>, kind: TokenType) -> Self {
        let ops::Range { start, end } = span;
        Self { start, end, kind }
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
                Say => String, // TODO: 自定义渲染跨行和文本拓展
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
                Intro => String, // TODO: 自定义渲染跨行
                MiniAvatar => Regex,
                SetTextbox => EnumMember,
                FilmMode => EnumMember,

                // 场景与分支
                CallScene => Regex,
                ChangeScene => Regex,
                Choose => String, // TODO: 自定义渲染选项和场景
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
