//! 语句模型

use std::{fmt, result};

use derive_more::{From, Into, TryInto};

pub use error::*;
pub use primary::*;
#[cfg(feature = "lsp")]
pub use scene::*;
pub use statement::*;
pub use webgal_sentence_macro::Sentence; // 重新导出方便使用

mod error;
mod primary;
#[cfg(feature = "lsp")]
mod scene;
mod statement;

/// 可从初级语句 [`PrimarySentence`] 构建的语句类型
pub trait FromPrimary: Sized {
    /// 从初级语句构建
    fn from_primary(primary: &PrimarySentence, errors: &mut Vec<Error>) -> Self;
}

/// 语句
///
/// # Behavior
/// * 语句不携带注释信息, 序列化时以 `;` 结尾.
///
/// # Performance
/// 为防止枚举膨胀, 部分枚举项存储在堆上.
/// 实际使用中其它语句基本都是 `say`, 可忽略小语句的内存浪费.
#[derive(Debug, Clone, PartialEq, PartialOrd, From, TryInto)]
pub enum Sentence {
    // 常规演出
    Say(SaySentence),
    ChangeBackground(Box<ChangeBackgroundSentence>),
    ChangeFigure(Box<ChangeFigureSentence>),
    Bgm(BgmSentence),
    PlayVideo(PlayVideoSentence),
    PlayEffect(PlayEffectSentence),
    // 舞台对象控制
    SetAnimation(SetAnimationSentence),
    SetComplexAnimation(SetComplexAnimationSentence),
    SetTransform(Box<SetTransformSentence>),
    SetTempAnimation(SetTempAnimationSentence),
    SetTransition(SetTransitionSentence),
    // 特殊演出
    PixiPerform(PixiPerformSentence),
    PixiInit(PixiInitSentence),
    Intro(IntroSentence),
    MiniAvatar(MiniAvatarSentence),
    SetTextbox(SetTextboxSentence),
    FilmMode(FilmModeSentence),
    // 场景与分支
    CallScene(CallSceneSentence),
    ChangeScene(ChangeSceneSentence),
    Choose(ChooseSentence),
    Label(LabelSentence),
    JumpLabel(JumpLabelSentence),
    // 鉴赏
    UnlockCg(UnlockCgSentence),
    UnlockBgm(UnlockBgmSentence),
    // 游戏控制
    GetUserInput(Box<GetUserInputSentence>),
    SetVar(SetVarSentence),
    ShowVars(ShowVarsSentence),
    Wait(WaitSentence),
    ApplyStyle(ApplyStyleSentence),
    CallSteam(CallSteamSentence),
    End(EndSentence),
    // 空白注释
    Comment(CommentSentence),
}

impl Sentence {
    /// 从语句字符串构建
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(line: &str) -> SentenceOutput {
        let primary = PrimarySentence::from_str(line);
        let mut errors = Vec::new();
        let sentence = Self::from_primary(&primary, &mut errors);
        SentenceOutput { sentence, errors }
    }

    pub fn get_command(&self) -> &'static str {
        use Sentence::*;

        macro_rules! get_command_match {
            ($sentence:ident: {$($variant:ident),* $(,)?}) => {{
                match $sentence {
                    $($variant(sentence) => sentence.get_command(),)*
                }
            }};
        }

        get_command_match! {
            self: {
                // 常规演出
                Say, ChangeBackground, ChangeFigure, Bgm, PlayVideo, PlayEffect,
                // 舞台对象控制
                SetAnimation, SetComplexAnimation, SetTransform, SetTempAnimation, SetTransition,
                // 特殊演出
                PixiPerform, PixiInit, Intro, MiniAvatar, SetTextbox, FilmMode,
                // 场景与分支
                CallScene, ChangeScene, Choose, Label, JumpLabel,
                // 鉴赏
                UnlockCg, UnlockBgm,
                // 游戏控制
                GetUserInput, SetVar, ShowVars, Wait, ApplyStyle, CallSteam, End,
                // 空白注释
                Comment,
            }
        }
    }

    pub fn is_say(&self) -> bool {
        matches!(self, Self::Say(_))
    }
}

impl FromPrimary for Sentence {
    fn from_primary(primary: &PrimarySentence, errors: &mut Vec<Error>) -> Self {
        macro_rules! from_primary_match {
            (
                ($primary:ident, $errors:ident):
                {$($pattern:pat $(if $guard:expr)? => $sentence:ty),* $(,)?}
            ) => {
                match $primary.command {
                    $($pattern $(if $guard)? => {
                        <$sentence>::from_primary($primary, $errors).into()
                    })*
                }
            };
        }

        from_primary_match! {
            (primary, errors): {
                // 常规演出
                _ if primary.content.is_none() && !primary.arguments.is_empty() => SaySentence,
                "changeBg" => ChangeBackgroundSentence,
                "changeFigure" => ChangeFigureSentence,
                "bgm" => BgmSentence,
                "playVideo" => PlayVideoSentence,
                "playEffect" => PlayEffectSentence,
                // 舞台对象控制
                "setAnimation" => SetAnimationSentence,
                "setComplexAnimation" => SetComplexAnimationSentence,
                "setTransform" => SetTransformSentence,
                "setTempAnimation" => SetTempAnimationSentence,
                "setTransition" => SetTransitionSentence,
                // 特殊演出
                "pixiPerform" => PixiPerformSentence,
                "pixiInit" => PixiInitSentence,
                "intro" => IntroSentence,
                "miniAvatar" => MiniAvatarSentence,
                "setTextbox" => SetTextboxSentence,
                "filmMode" => FilmModeSentence,
                // 场景与分支
                "callScene" => CallSceneSentence,
                "changeScene" => ChangeSceneSentence,
                "choose" => ChooseSentence,
                "label" => LabelSentence,
                "jumpLabel" => JumpLabelSentence,
                // 鉴赏
                "unlockCg" => UnlockCgSentence,
                "unlockBgm" => UnlockBgmSentence,
                // 游戏控制
                "getUserInput" => GetUserInputSentence,
                "setVar" => SetVarSentence,
                "showVars" => ShowVarsSentence,
                "wait" => WaitSentence,
                "applyStyle" => ApplyStyleSentence,
                "callSteam" => CallSteamSentence,
                "end" => EndSentence,
                // 其它
                "" if primary.content.is_none() && primary.arguments.is_empty() => CommentSentence,
                _ => SaySentence,
            }
        }
    }
}

impl fmt::Display for Sentence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Sentence::*;

        macro_rules! display_match {
            ($sentence:ident: {$($variant:ident),* $(,)?}) => {{
                match $sentence {
                    $($variant(sentence) => sentence.fmt(f),)*
                }
            }};
        }

        display_match! {
            self: {
                // 常规演出
                Say, ChangeBackground, ChangeFigure, Bgm, PlayVideo, PlayEffect,
                // 舞台对象控制
                SetAnimation, SetComplexAnimation, SetTransform, SetTempAnimation, SetTransition,
                // 特殊演出
                PixiPerform, PixiInit, Intro, MiniAvatar, SetTextbox, FilmMode,
                // 场景与分支
                CallScene, ChangeScene, Choose, Label, JumpLabel,
                // 鉴赏
                UnlockCg, UnlockBgm,
                // 游戏控制
                GetUserInput, SetVar, ShowVars, Wait, ApplyStyle, CallSteam, End,
                // 空白注释
                Comment,
            }
        }
    }
}

impl Default for Sentence {
    fn default() -> Self {
        Self::Comment(CommentSentence {})
    }
}

macro_rules! impl_from_and_try_into_for_boxed_sentence {
    ($t:ty) => {
        impl ::std::convert::From<$t> for Sentence {
            fn from(value: $t) -> Self {
                Box::new(value).into()
            }
        }

        impl ::std::convert::TryFrom<Sentence> for $t {
            type Error = ::derive_more::TryIntoError<Sentence>;

            fn try_from(value: Sentence) -> ::std::result::Result<Self, Self::Error> {
                let sentence: Box<_> = value.try_into()?;
                Ok(*sentence)
            }
        }
    };
}

impl_from_and_try_into_for_boxed_sentence!(ChangeBackgroundSentence);
impl_from_and_try_into_for_boxed_sentence!(ChangeFigureSentence);
impl_from_and_try_into_for_boxed_sentence!(SetTransformSentence);
impl_from_and_try_into_for_boxed_sentence!(GetUserInputSentence);

#[derive(Debug, From, Into)]
pub struct SentenceOutput {
    pub sentence: Sentence,
    pub errors: Vec<Error>,
}

impl SentenceOutput {
    pub fn has_error(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn ok(self) -> result::Result<Sentence, Vec<Error>> {
        if self.has_error() {
            Ok(self.sentence)
        } else {
            Err(self.errors)
        }
    }
}
