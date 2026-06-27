use std::fmt::{self, Write};

use webgal_sentence_macro::Sentence;

use crate::{
    element::{
        AnimationList, Color, Ease, FigureId, FigureSide, FontSize, Forward, IntroAnimation,
        Live2dBlink, Live2dBounds, Live2dFocus, ObjectId, Sustain, Transform,
    },
    sentence::{Error, FromPrimary, PrimarySentence},
    util::{write_joined, write_joined_with},
};

// -------- 常规演出 --------

/// 普通对话语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaySentence {
    pub content: Vec<String>,
    pub speaker: Option<String>,
    // 效果
    pub vocal: Option<String>,
    pub figure: Option<FigureId>,
    pub font_size: FontSize,
    // 控制
    pub concat: bool,
    pub notend: bool,
    pub when: Option<String>,
}

/// 切换背景语句
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Sentence)]
#[sentence(command = "changeBg", validate = Self::validate)]
pub struct ChangeBackgroundSentence {
    #[sentence(content)]
    pub background: String,
    // 效果
    pub transform: Option<Transform>,
    pub enter: Option<String>,
    pub exit: Option<String>,
    #[sentence(default)]
    pub ease: Ease,
    // 鉴赏
    pub unlockname: Option<String>,
    #[sentence(require = ["unlockname"])]
    pub series: Option<String>,
    // 控制
    pub duration: Option<usize>,
    #[sentence(rename = "enterDuration", require = ["enter"])]
    pub enter_duration: Option<usize>,
    #[sentence(rename = "exitDuration", require = ["exit"])]
    pub exit_duration: Option<usize>,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 切换立绘语句
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Sentence)]
#[sentence(
    command = "changeFigure",
    validate = Self::validate,
    obsolete = {
        "clear": "设置退场请直接输入空立绘",
        "none": "设置退场请直接输入空立绘",
        "animationFlag": "保留参数",
    }
)]
pub struct ChangeFigureSentence {
    #[sentence(content)]
    pub figure: String,
    #[sentence(variant = { "left": Left, "right": Right })]
    pub side: FigureSide,
    pub id: Option<String>,
    // 图像立绘
    #[sentence(rename = "mouthOpen")]
    pub mouth_open: Option<String>,
    #[sentence(rename = "mouthHalfOpen")]
    pub mouth_half_open: Option<String>,
    #[sentence(rename = "mouthClose")]
    pub mouth_close: Option<String>,
    #[sentence(rename = "eyesOpen")]
    pub eyes_open: Option<String>,
    #[sentence(rename = "eyesClose")]
    pub eyes_close: Option<String>,
    // Live2D / Spine 立绘
    pub skin: Option<String>,
    pub motion: Option<String>,
    pub expression: Option<String>,
    pub bounds: Option<Live2dBounds>,
    pub blink: Option<Live2dBlink>,
    pub focus: Option<Live2dFocus>,
    // 效果
    pub transform: Option<Transform>,
    pub enter: Option<String>,
    pub exit: Option<String>,
    #[sentence(default)]
    pub ease: Ease,
    #[sentence(rename = "zIndex")]
    pub z_index: Option<usize>,
    // 控制
    pub duration: Option<usize>,
    #[sentence(rename = "enterDuration", require = ["enter"])]
    pub enter_duration: Option<usize>,
    #[sentence(rename = "exitDuration", require = ["exit"])]
    pub exit_duration: Option<usize>,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 背景音乐语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "bgm",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct BgmSentence {
    #[sentence(content)]
    pub bgm: String,
    // 效果
    pub volume: Option<usize>,
    pub enter: Option<usize>,
    // 鉴赏
    pub unlockname: Option<String>,
    #[sentence(require = ["unlockname"])]
    pub series: Option<String>,
    // 控制
    pub when: Option<String>,
}

/// 播放视频语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "playVideo",
    obsolete = {
        "next": "语句自带连续执行效果",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct PlayVideoSentence {
    #[sentence(content)]
    pub video: String,
    // 控制
    #[sentence(rename = "skipOff")]
    pub skip_off: bool,
    pub when: Option<String>,
}

/// 效果声音语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "playEffect",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct PlayEffectSentence {
    #[sentence(content)]
    pub vocal: String,
    pub id: Option<String>,
    // 效果
    pub volume: Option<usize>,
    // 控制
    pub when: Option<String>,
}

// -------- 舞台对象控制 --------

/// 调用动画语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(command = "setAnimation")]
pub struct SetAnimationSentence {
    #[sentence(content)]
    pub animation: String,
    pub target: Option<ObjectId>,
    // 效果
    #[sentence(rename = "writeDefault")]
    pub write_default: bool,
    // 控制
    #[sentence(variant = { "keep": Keep, "parallel": Parallel })]
    pub sustain: Sustain,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 复杂动画语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(command = "setComplexAnimation")]
pub struct SetComplexAnimationSentence {
    #[sentence(content)]
    pub animation: String,
    pub target: Option<ObjectId>,
    // 效果
    #[sentence(rename = "writeDefault")]
    pub write_default: bool,
    // 控制
    pub duration: Option<usize>,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 单段动画语句
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Sentence)]
#[sentence(command = "setTransform", validate = Self::validate)]
pub struct SetTransformSentence {
    #[sentence(content)]
    pub transform: Transform,
    pub target: Option<ObjectId>,
    // 效果
    #[sentence(rename = "writeDefault")]
    pub write_default: bool,
    #[sentence(default)]
    pub ease: Ease,
    // 控制
    pub duration: Option<usize>,
    #[sentence(variant = { "keep": Keep, "parallel": Parallel })]
    pub sustain: Sustain,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 多段动画语句
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Sentence)]
#[sentence(command = "setTempAnimation", validate = Self::validate)]
pub struct SetTempAnimationSentence {
    #[sentence(content)]
    pub animation: AnimationList,
    pub target: Option<ObjectId>,
    // 效果
    #[sentence(rename = "writeDefault")]
    pub write_default: bool,
    // 控制
    #[sentence(variant = { "keep": Keep, "parallel": Parallel })]
    pub sustain: Sustain,
    #[sentence(variant = { "continue": Continue, "next": Next })]
    pub forward: Forward,
    pub when: Option<String>,
}

/// 进出场动画语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "setTransition",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct SetTransitionSentence {
    pub target: Option<ObjectId>,
    pub enter: Option<String>,
    pub exit: Option<String>,
    // 控制
    pub when: Option<String>,
}

// -------- 特殊演出 --------

/// 使用特效语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "pixiPerform",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct PixiPerformSentence {
    #[sentence(content)]
    pub effect: String,
    // 控制
    pub when: Option<String>,
}

/// 清除特效语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "pixiInit",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct PixiInitSentence {
    // 控制
    pub when: Option<String>,
}

/// 全屏文字语句
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Sentence)]
#[sentence(
    command = "intro",
    validate = Self::validate,
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct IntroSentence {
    #[sentence(content, serialize_with = display_text, deserialize_with = parse_text)]
    pub content: Vec<String>,
    // 效果
    #[sentence(rename = "fontSize", default)]
    pub font_size: FontSize,
    #[sentence(
        rename = "fontColor",
        serialize_with = Color::fmt_css_rgba,
        deserialize_with = parse_color_css_rgba,
    )]
    pub font_color: Option<Color>,
    #[sentence(
        rename = "backgroundColor",
        serialize_with = Color::fmt_css_rgba,
        deserialize_with = parse_color_css_rgba,
    )]
    pub background_color: Option<Color>,
    #[sentence(rename = "backgroundImage")]
    pub background_image: Option<String>,
    #[sentence(default)]
    pub animation: IntroAnimation,
    // 控制
    #[sentence(rename = "delayTime")]
    pub delay: Option<usize>,
    pub hold: bool,
    #[sentence(rename = "useForward")]
    pub use_forward: bool,
    pub when: Option<String>,
}

/// 角落头像语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "miniAvatar",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct MiniAvatarSentence {
    #[sentence(content)]
    pub avatar: String,
    // 控制
    pub when: Option<String>,
}

/// 文本显示语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "setTextbox",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct SetTextboxSentence {
    #[sentence(content, serialize_with = display_show, deserialize_with = parse_show)]
    pub show: bool,
    // 控制
    pub when: Option<String>,
}

/// 电影模式语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "filmMode",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct FilmModeSentence {
    #[sentence(content, serialize_with = display_enable, deserialize_with = parse_enable)]
    pub enable: bool,
    // 控制
    pub when: Option<String>,
}

// -------- 场景与分支 --------

/// 调用场景语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "callScene",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct CallSceneSentence {
    #[sentence(content)]
    pub scene: String,
    // 控制
    pub when: Option<String>,
}

/// 切换场景语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "changeScene",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct ChangeSceneSentence {
    #[sentence(content)]
    pub scene: String,
    // 控制
    pub when: Option<String>,
}

/// 分支选择语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "choose",
    validate = Self::validate,
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct ChooseSentence {
    #[sentence(content, serialize_with = display_choices, deserialize_with = parse_choices)]
    pub choices: Vec<(String, String)>,
    // 控制
    #[sentence(rename = "defaultChoice")]
    pub default_choice: Option<usize>,
    pub when: Option<String>,
}

/// 标签语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "label",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct LabelSentence {
    #[sentence(content)]
    pub label: String,
    // 控制
    pub when: Option<String>,
}

/// 跳转标签语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "jumpLabel",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct JumpLabelSentence {
    #[sentence(content)]
    pub label: String,
    // 控制
    pub when: Option<String>,
}

// -------- 鉴赏 --------

/// 鉴赏图片语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "unlockCg",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct UnlockCgSentence {
    #[sentence(content)]
    pub image: String,
    // 鉴赏
    pub name: Option<String>,
    #[sentence(require = ["name"])]
    pub series: Option<String>,
    // 控制
    pub when: Option<String>,
}

/// 鉴赏音乐语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "unlockBgm",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct UnlockBgmSentence {
    #[sentence(content)]
    pub bgm: String,
    // 鉴赏
    pub name: Option<String>,
    #[sentence(require = ["name"])]
    pub series: Option<String>,
    // 控制
    pub when: Option<String>,
}

// -------- 游戏控制 --------

/// 获取输入语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "getUserInput",
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct GetUserInputSentence {
    #[sentence(content)]
    pub variable: String,
    // 效果
    pub title: Option<String>,
    #[sentence(rename = "buttonText")]
    pub button_text: Option<String>,
    pub default_value: Option<String>,
    // 校验
    pub rule: Option<String>,
    #[sentence(rename = "ruleFlag", require = ["rule"])]
    pub rule_flag: Option<String>,
    #[sentence(rename = "ruleText", require = ["rule"])]
    pub rule_text: Option<String>,
    #[sentence(rename = "ruleButtonText", require = ["rule"])]
    pub rule_button_text: Option<String>,
    // 控制
    pub when: Option<String>,
}

/// 设置变量语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "setVar",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct SetVarSentence {
    #[sentence(content)]
    pub expression: String,
    pub global: bool,
    // 控制
    pub when: Option<String>,
}

/// 显示变量语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "showVars",
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct ShowVarsSentence {
    // 控制
    pub when: Option<String>,
}

/// 等待语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "wait",
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "语句自带连续执行效果",
    }
)]
pub struct WaitSentence {
    #[sentence(content)]
    pub duration: usize,
    // 控制
    pub when: Option<String>,
}

/// 应用样式语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "applyStyle",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct ApplyStyleSentence {
    #[sentence(
        content,
        serialize_with = display_style_applications,
        deserialize_with = parse_style_applications,
    )]
    pub applications: Vec<(String, String)>,
    // 控制
    pub when: Option<String>,
}

/// 调用 Steam 语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "callSteam",
    obsolete = {
        "next": "语句自带同步执行效果",
        "continue": "语句自带同步执行效果",
    }
)]
pub struct CallSteamSentence {
    #[sentence(rename = "achivementId")]
    pub achivement_id: Option<String>,
    // 控制
    pub when: Option<String>,
}

/// 结束游戏语句
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(
    command = "end",
    obsolete = {
        "next": "控制的演出时序无意义",
        "continue": "控制的演出时序无意义",
    }
)]
pub struct EndSentence {
    // 控制
    pub when: Option<String>,
}

// -------- 空白注释 --------

/// 空白注释语句
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Sentence)]
#[sentence(command = "")]
pub struct CommentSentence {} // 单元结构体暂时不可用

// -------- 校验 --------

impl ChangeBackgroundSentence {
    pub fn validate(&self, primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if let Some(transform) = &self.transform
            && let Err(error) = transform.validate()
            && let Some((index, _)) = primary.get_argument("transform")
        {
            errors.push(Error::ArgumentType(index, error));
        }
    }
}

impl ChangeFigureSentence {
    pub fn validate(&self, primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if let Some(transform) = &self.transform
            && let Err(error) = transform.validate()
            && let Some((index, _)) = primary.get_argument("transform")
        {
            errors.push(Error::ArgumentType(index, error));
        }

        // 空立绘
        if matches!(self.figure.as_str(), "" | "none") {
            if self.motion.is_some()
                && let Some((index, _)) = primary.get_argument("motion")
            {
                errors.push(Error::ArgumentMissingDependencies(index, vec!["figure"]));
            }
            if self.expression.is_some()
                && let Some((index, _)) = primary.get_argument("expression")
            {
                errors.push(Error::ArgumentMissingDependencies(index, vec!["figure"]));
            }
        }

        // 立绘类型冲突
        let figure_type_conflict = {
            let is_image = self.mouth_open.is_some()
                || self.mouth_half_open.is_some()
                || self.mouth_close.is_some()
                || self.eyes_open.is_some()
                || self.eyes_close.is_some();
            let is_live2d = self.expression.is_some()
                || self.bounds.is_some()
                || self.blink.is_some()
                || self.focus.is_some();
            let is_spine = self.skin.is_some();
            let is_live2d_or_spine = is_live2d || is_spine || self.motion.is_some();
            (is_image && is_live2d_or_spine) || (is_live2d && is_spine)
        };
        if figure_type_conflict {
            errors.push(Error::ContentType(anyhow::anyhow!(
                "立绘不能同时为图像 / Live2D / Spine"
            )));
        }
    }
}

impl SetTransformSentence {
    pub fn validate(&self, _primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if let Err(error) = self.transform.validate() {
            errors.push(Error::ContentType(error));
        }
    }
}

impl SetTempAnimationSentence {
    pub fn validate(&self, _primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if let Err(error) = self.animation.validate() {
            errors.push(Error::ContentType(error));
        }
    }
}

impl IntroSentence {
    pub fn validate(&self, primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if self.background_color.is_some()
            && self.background_image.is_some()
            && let Some((index, _)) = primary.get_argument("backgroundColor")
        {
            errors.push(Error::ArgumentType(
                index,
                anyhow::anyhow!("`backgroundColor` 与 `backgroundImage` 参数不能同时出现"),
            ));
        }
    }
}

impl ChooseSentence {
    pub fn validate(&self, primary: &PrimarySentence, errors: &mut Vec<Error>) {
        if let Some(default_choice) = self.default_choice
            && !(1..self.choices.len()).contains(&default_choice)
            && let Some((index, _)) = primary.get_argument("defaultChoice")
        {
            errors.push(Error::ArgumentType(
                index,
                anyhow::anyhow!(
                    "`defaultChoice` 参数出界, 其范围是 [1..{})",
                    self.choices.len()
                ),
            ));
        }
    }
}

// -------- 对话 --------

impl SaySentence {
    pub fn get_command(&self) -> &'static str {
        "say"
    }

    // /// 对话是否为空, 若空则需要 `-saySpaceHolder`
    // fn is_empty(&self) -> bool {
    //     *self == Self::default()
    // }
}

impl FromPrimary for SaySentence {
    fn from_primary(primary: &PrimarySentence, errors: &mut Vec<Error>) -> Self {
        let PrimarySentence {
            command,
            content,
            arguments,
            ..
        } = primary;

        let (content, mut speaker) = match content {
            Some(content) if *command == "say" => (content, None),
            Some(content) => (content, Some(command.to_string())),
            None => (command, None),
        };
        let (content, _) = parse_text(content.trim()); // 不会出错

        let mut space_holder = false;
        let mut vocal = None;
        let mut figure = None;
        let mut font_size = None;
        let mut concat = None;
        let mut notend = None;
        let mut when = None;

        for (i, &(name, value)) in arguments.iter().enumerate() {
            match name {
                "speaker" => {
                    if speaker.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    speaker = Some(value.unwrap_or("true").to_string())
                }
                "vocal" => {
                    if vocal.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    vocal = Some(value.unwrap_or("true").to_string());
                }
                "left" | "center" | "right" => {
                    if figure.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                    }
                    figure = Some(FigureId::Side(name.parse().unwrap()));
                }
                "figureId" => {
                    if let Some(figure) = &figure {
                        errors.push(Error::ArgumentRepeated(i));
                        if figure.is_side() {
                            continue;
                        }
                    }
                    figure = Some(FigureId::Id(value.unwrap_or("true").to_string()))
                }
                "fontSize" => {
                    if font_size.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    font_size = Some(value.unwrap_or("true").parse::<FontSize>().unwrap_or_else(
                        |error| {
                            errors.push(Error::ArgumentType(i, error.into()));
                            FontSize::default()
                        },
                    ));
                }
                "concat" => {
                    if concat.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    concat = Some(!matches!(value, Some("") | Some("false") | Some("0")));
                }
                "notend" => {
                    if notend.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    notend = Some(!matches!(value, Some("") | Some("false") | Some("0")));
                }
                "when" => {
                    if when.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    when = Some(value.unwrap_or("true").to_string());
                }
                "next" | "continue" => {
                    errors.push(Error::ArgumentObsolete(i, "控制的演出时序无意义"));
                }
                "sayPlaceHolder" => {
                    if space_holder {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    space_holder = true;
                }
                _ => {
                    if vocal.is_some() {
                        errors.push(Error::ArgumentRepeated(i));
                        continue;
                    }
                    vocal = Some(name.to_string());
                }
            }
        }

        Self {
            content,
            speaker,
            vocal,
            figure,
            font_size: font_size.unwrap_or_default(),
            concat: concat.unwrap_or(false),
            notend: notend.unwrap_or(false),
            when,
        }
    }
}

impl fmt::Display for SaySentence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            content,
            speaker,
            vocal,
            figure,
            font_size,
            concat,
            notend,
            when,
        } = self;
        let mut need_space_holder = true;

        if let Some(speaker) = speaker {
            need_space_holder = false;
            write!(f, "{speaker}:")?;
        }
        need_space_holder &= content.is_empty();
        display_text(content, f)?;

        if let Some(vocal) = vocal {
            need_space_holder = false;
            display_vocal(vocal, f)?;
        }
        if let Some(figure) = figure {
            need_space_holder = false;
            match figure {
                FigureId::Id(id) => write!(f, " -figureId={id}")?,
                &FigureId::Side(side) => write!(f, " -{side}")?,
            }
        }
        if *font_size != FontSize::default() {
            need_space_holder = false;
            write!(f, " -fontSize={font_size}")?;
        }
        if *concat {
            need_space_holder = false;
            f.write_str(" -concat")?;
        }
        if *notend {
            need_space_holder = false;
            f.write_str(" -notend")?;
        }
        if let Some(when) = when {
            need_space_holder = false;
            write!(f, " -when={when}")?;
        }

        if need_space_holder {
            f.write_str(" -sayPlaceHolder")?;
        }
        f.write_char(';')
    }
}

// -------- 序列化与反序列化 --------

fn display_vocal(vocal: &str, f: &mut fmt::Formatter) -> fmt::Result {
    match vocal {
        "speaker" | "vocal" | "left" | "center" | "right" | "figureId" | "fontSize" | "next"
        | "continue" | "concat" | "notend" | "when" => write!(f, " -vocal={vocal}"),
        _ => write!(f, " -{vocal}"),
    }
}

fn parse_text(text: &str) -> (Vec<String>, Option<anyhow::Error>) {
    (
        if text.is_empty() {
            Vec::default()
        } else {
            text.split('|').map(str::to_string).collect()
        },
        None,
    )
}

fn display_text(text: &[String], f: &mut fmt::Formatter) -> fmt::Result {
    write_joined(f, text.iter(), "|")
}

fn parse_color_css_rgba(s: &str) -> (Color, Option<anyhow::Error>) {
    match Color::from_str_rgba(s) {
        Some(color) => (color, None),
        None => (
            Color::default(),
            Some(anyhow::anyhow!(concat!(
                "颜色应为 `rgba(R,G,B,A)` 的格式, ",
                "颜色在 [0..256) 间, 透明度在 [0,1] 间"
            ))),
        ),
    }
}

fn parse_show(state: &str) -> (bool, Option<anyhow::Error>) {
    (state != "hide", None)
}

fn display_show(show: &bool, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(if *show { "show" } else { "hide" })
}

fn parse_enable(state: &str) -> (bool, Option<anyhow::Error>) {
    (!state.is_empty() && state != "none", None)
}

fn display_enable(enable: &bool, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(if *enable { "enable" } else { "none" })
}

fn parse_choices(choices: &str) -> (Vec<(String, String)>, Option<anyhow::Error>) {
    if choices.is_empty() {
        return (Vec::default(), Some(anyhow::anyhow!("没有任何选项")));
    }
    let mut miss_scene = false;
    let choices = choices
        .split('|')
        .map(|choice| {
            let (prompt, scene) = choice.split_once(':').unwrap_or_else(|| {
                miss_scene = true;
                (choice, "")
            });
            (prompt.trim().to_string(), scene.trim().to_string())
        })
        .collect();
    (
        choices,
        miss_scene.then(|| anyhow::anyhow!("选项缺少跳转目标, 请考虑使用空字符串占位")),
    )
}

fn display_choices(choices: &[(String, String)], f: &mut fmt::Formatter) -> fmt::Result {
    write_joined_with(f, choices.iter(), "|", |(prompt, scene), f| {
        write!(f, "{prompt}:{scene}")
    })
}

fn parse_style_applications(applications: &str) -> (Vec<(String, String)>, Option<anyhow::Error>) {
    if applications.is_empty() {
        return (Vec::default(), Some(anyhow::anyhow!("没有任何样式")));
    }
    let mut miss_current = false;
    let applications = applications
        .split('|')
        .map(|application| {
            let (previous, current) = application.split_once(':').unwrap_or_else(|| {
                miss_current = true;
                (application, "")
            });
            (previous.trim().to_string(), current.trim().to_string())
        })
        .collect();
    (
        applications,
        miss_current.then(|| anyhow::anyhow!("选项缺少新样式, 请考虑使用空字符串占位")),
    )
}

fn display_style_applications(
    appliactions: &[(String, String)],
    f: &mut fmt::Formatter,
) -> fmt::Result {
    write_joined_with(f, appliactions.iter(), ",", |(previous, current), f| {
        write!(f, "{previous}->{current}")
    })
}

#[cfg(test)]
mod tests {
    // This module is partially generated by AI.

    use crate::sentence::{Sentence, split_comment};

    use super::*;

    #[test]
    fn roundtrip() {
        let original = include_str!("../../tests/fixtures/scene/roundtrip-original.txt");
        let expected = include_str!("../../tests/fixtures/scene/roundtrip-expected.txt");
        for (i, (original, expected)) in original.lines().zip(expected.lines()).enumerate() {
            let sentence = Sentence::from_str(original)
                .ok()
                .unwrap_or_else(|_| panic!("parse failed: {}", i + 1));
            let comment = split_comment(original).unwrap_or_default().1.trim();
            let display = format!("{sentence}{}", format!(" {comment}").trim_end());
            assert_eq!(display, expected, "display differ: {}", i + 1);
        }
    }

    // -------- say --------

    #[test]
    fn say_basic_content_only() {
        let s = "Just some content;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.content, vec!["Just some content".to_string()]);
                assert_eq!(say.speaker, None);
                assert_eq!(say.to_string(), s);
            }
            _ => panic!("expected SaySentence"),
        }
    }

    #[test]
    fn say_sentence_with_speaker_and_multiline() {
        let original = "Alice:Hello|Welcome|Bye;";
        let output = Sentence::from_str(original);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.speaker, Some("Alice".to_string()));
                assert_eq!(
                    say.content,
                    vec![
                        "Hello".to_string(),
                        "Welcome".to_string(),
                        "Bye".to_string()
                    ]
                );
                assert_eq!(say.to_string(), original);
            }
            _ => panic!("期望 SaySentence"),
        }
    }

    #[test]
    fn say_speaker_via_colon() {
        // 有 `speaker` 时, 格式为 `speaker:content`
        let s = "Tom:Hello;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.content, vec!["Hello".to_string()]);
                assert_eq!(say.speaker, Some("Tom".to_string()));
                assert_eq!(say.to_string(), s);
            }
            _ => panic!(),
        }

        // 空说话人 (只有冒号)
        let s2 = ":;";
        let output2 = Sentence::from_str(s2);
        assert!(output2.errors.is_empty());
        match output2.sentence {
            Sentence::Say(say) => {
                assert!(say.content.is_empty());
                assert_eq!(say.speaker, Some("".to_string()));
                assert_eq!(say.to_string(), s2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn say_speaker_via_explicit_parameter() {
        let s = "content -speaker=Tracy;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.content, vec!["content".to_string()]);
                assert_eq!(say.speaker, Some("Tracy".to_string()));
                assert_eq!(say.to_string(), "Tracy:content;"); // 按语法糖序列化
            }
            _ => panic!(),
        }
    }

    #[test]
    fn say_trailing_space_in_command() {
        // 命令尾部空格: 整个 `contentWithSpace ` 作为 command
        let s = "contentWithSpace ;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.content, vec!["contentWithSpace".to_string()]);
                assert_eq!(say.speaker, None);
                assert_eq!(say.to_string(), "contentWithSpace;");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn say_vocal_unrecognized() {
        // 未识别的参数名直接作为 vocal (不带等号)
        let s = "Hello -voice;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.content, vec!["Hello".to_string()]);
                assert_eq!(say.vocal, Some("voice".to_string()));
                assert_eq!(say.to_string(), s);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn say_vocal_collision_with_keyword() {
        // vocal 值与保留字冲突时, 序列化应使用 `-vocal=value`
        let s = "Hello -vocal=speaker;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        match output.sentence {
            Sentence::Say(say) => {
                assert_eq!(say.vocal, Some("speaker".to_string()));
                assert_eq!(say.to_string(), "Hello -vocal=speaker;");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn say_placeholder() {
        // 完全空的对话 (无内容, 无参数) 应解析为默认 SaySentence
        let s = " -sayPlaceHolder;";
        let output = Sentence::from_str(s);
        assert!(output.errors.is_empty());
        assert_eq!(output.sentence, Sentence::Say(Default::default()));
        // 序列化应产生相同的占位符
        assert_eq!(output.sentence.to_string(), s);
    }

    // -------- parse --------

    #[test]
    fn choose_sentence_multiple_options() {
        // 多个选项
        let original = "choose:Option A:scene_a|Option B:scene_b|Option C:scene_c;";
        let output = Sentence::from_str(original);
        assert!(output.errors.is_empty(), "解析出错: {:?}", output.errors);
        match output.sentence {
            Sentence::Choose(choose) => {
                assert_eq!(
                    choose.choices,
                    vec![
                        ("Option A".to_string(), "scene_a".to_string()),
                        ("Option B".to_string(), "scene_b".to_string()),
                        ("Option C".to_string(), "scene_c".to_string()),
                    ],
                    "解析后的 choices 不匹配"
                );
                let serialized = choose.to_string();
                assert_eq!(serialized, original, "序列化后应与原字符串一致");
            }
            _ => panic!("期望 ChooseSentence"),
        }
    }

    // -------- require --------

    #[test]
    fn require_series_needs_unlockname() {
        // changeBg 语句中 series 依赖 unlockname
        let s = "changeBg:bg.png -series=summer;";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentMissingDependencies(_, _)))
        );
        // 正确使用时无错误
        let s_ok = "changeBg:bg.png -unlockname=pic -series=summer;";
        let output_ok = Sentence::from_str(s_ok);
        assert!(output_ok.errors.is_empty());
    }

    #[test]
    fn require_enter_duration_needs_enter() {
        // enterDuration 依赖 enter
        let s = "changeBg:bg.png -enterDuration=500;";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentMissingDependencies(_, _)))
        );
        // 正确使用
        let s_ok = "changeBg:bg.png -enter=fade -enterDuration=500;";
        let output_ok = Sentence::from_str(s_ok);
        assert!(output_ok.errors.is_empty());
    }

    #[test]
    fn require_rule_flag_needs_rule() {
        // getUserInput 中 ruleFlag 依赖 rule
        let s = "getUserInput:var -ruleFlag=must;";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentMissingDependencies(_, _)))
        );
        let s_ok = "getUserInput:var -rule=number -ruleFlag=must;";
        let output_ok = Sentence::from_str(s_ok);
        assert!(output_ok.errors.is_empty());
    }

    // -------- validate --------

    #[test]
    fn validate_transform_checks_alpha() {
        // setTransform 中 transform 的 alpha 越界应产生 ContentType 错误
        let s = "setTransform:{\"alpha\":2.0};";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ContentType(_)))
        );
    }

    #[test]
    fn validate_choose_default_choice_out_of_range() {
        // choose 语句的 defaultChoice 超出选项数量
        let s = "choose:Option A:scene_a|Option B:scene_b -defaultChoice=3;";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentType(_, _)))
        );
    }

    // -------- obsolete --------

    #[test]
    fn obsolete_argument_detection() {
        // changeFigure 中 `-clear` 已弃用
        let s = "changeFigure:model.json -clear;";
        let output = Sentence::from_str(s);
        assert!(!output.errors.is_empty());
        assert!(
            output
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentObsolete(_, _)))
        );
        // 同样测试 bgm 中的 `-next`
        let s2 = "bgm:music.flac -next;";
        let output2 = Sentence::from_str(s2);
        assert!(!output2.errors.is_empty());
        assert!(
            output2
                .errors
                .iter()
                .any(|e| matches!(e, Error::ArgumentObsolete(_, _)))
        );
    }
}
