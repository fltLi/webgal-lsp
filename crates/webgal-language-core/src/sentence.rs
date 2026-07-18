//! 语句模型

use std::{fmt, result};

use derive_more::{From, Into, TryInto};
#[cfg(feature = "serde")]
use serde::Serialize;

use crate::element::Forward;

pub use error::*;
pub use primary::*;
pub use scene::*;
pub use statement::*;
pub use webgal_sentence_macro::Sentence; // 重新导出方便使用 // 无法导出

mod error;
mod primary;
mod scene;
mod statement;

/// 语句辅助方法
pub trait SentenceExt {
    /// 语句类型
    fn command(&self) -> &'static str;

    /// 执行时序
    fn forward(&self) -> Forward;

    // /// 触发条件
    // fn condition(&self) -> Option<&str>;

    // /// 关联资源
    // ///
    // /// # Returns
    // /// 资源类型和相对根的路径 (对于立绘可能包含类型后缀).
    // fn resources(&self) -> Vec<(ResourceKind, &str)> {
    //     Vec::default()
    // }
}

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
#[cfg_attr(
    feature = "serde",
    derive(Serialize),
    serde(tag = "command", rename_all = "camelCase")
)]
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

    pub fn is_say(&self) -> bool {
        matches!(self, Self::Say(_))
    }
}

impl Default for Sentence {
    fn default() -> Self {
        Self::Comment(CommentSentence {})
    }
}

impl SentenceExt for Sentence {
    fn command(&self) -> &'static str {
        crate::dispatch_sentence!(self.command())
    }

    fn forward(&self) -> Forward {
        crate::dispatch_sentence!(self.forward())
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
        crate::dispatch_sentence!(self.fmt(f))
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

impl From<SentenceOutput> for Sentence {
    fn from(value: SentenceOutput) -> Self {
        value.sentence
    }
}

/// 调用 [`Sentence`] 所有变体语句的方法
///
/// # Examples
/// ```
/// # use webgal_language_core::{dispatch_sentence, sentence::Sentence};
///
/// trait TypeName {
///     fn type_name(&self) -> &'static str;
/// }
///
/// impl<T> TypeName for T {
///     fn type_name(&self) -> &'static str {
///         std::any::type_name::<T>()
///     }
/// }
///
/// let sentence = Sentence::from_str("hello?").sentence;
///
/// assert_eq!(sentence.type_name(), "webgal_language_core::sentence::Sentence");
/// assert_eq!(
///     dispatch_sentence!(sentence.type_name()),
///     "webgal_language_core::sentence::statement::SaySentence",
/// );
/// ```
#[macro_export]
macro_rules! dispatch_sentence {
    ($sentence:ident.$method:ident($($argument:expr),* $(,)?)) => {
        match $sentence {
            // 常规演出
            $crate::sentence::Sentence::Say(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::ChangeBackground(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::ChangeFigure(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::Bgm(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::PlayVideo(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::PlayEffect(s) => s.$method($($argument),*),

            // 舞台对象控制
            $crate::sentence::Sentence::SetAnimation(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetComplexAnimation(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetTransform(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetTempAnimation(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetTransition(s) => s.$method($($argument),*),

            // 特殊演出
            $crate::sentence::Sentence::PixiPerform(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::PixiInit(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::Intro(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::MiniAvatar(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetTextbox(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::FilmMode(s) => s.$method($($argument),*),

            // 场景与分支
            $crate::sentence::Sentence::CallScene(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::ChangeScene(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::Choose(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::Label(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::JumpLabel(s) => s.$method($($argument),*),

            // 鉴赏
            $crate::sentence::Sentence::UnlockCg(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::UnlockBgm(s) => s.$method($($argument),*),

            // 游戏控制
            $crate::sentence::Sentence::GetUserInput(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::SetVar(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::ShowVars(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::Wait(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::ApplyStyle(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::CallSteam(s) => s.$method($($argument),*),
            $crate::sentence::Sentence::End(s) => s.$method($($argument),*),

            // 其它
            $crate::sentence::Sentence::Comment(s) => s.$method($($argument),*),
        }
    };
}
