//! 演出效果类型

use std::{fmt, str::FromStr};

use derive_more::{Deref, DerefMut, From, Into};
use serde::{Deserialize, Serialize};
use serde_with::{BoolFromInt, serde_as, skip_serializing_none};
use strum::{Display, EnumString};

use crate::{
    impl_display_for_serde_json, impl_from_str_for_from, impl_from_str_for_serde_json,
    util::try_validate,
};

// -------- 对象 --------

/// 对象引用, 包括舞台 + 背景 + 立绘
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectId {
    Stage,
    Background,
    Figure(FigureId),
}

impl ObjectId {
    /// 获取对象 ID
    pub fn get_id(&self) -> &str {
        match self {
            Self::Stage => "stage-main",
            Self::Background => "bg-main",
            Self::Figure(figure) => figure.get_id(),
        }
    }
}

impl<S: AsRef<str>> From<S> for ObjectId {
    fn from(value: S) -> Self {
        match value.as_ref() {
            "stage-main" => Self::Stage,
            "bg-main" => Self::Background,
            figure => Self::Figure(figure.into()),
        }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.get_id())
    }
}

impl_from_str_for_from!(ObjectId);

/// 立绘位置
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum FigureSide {
    #[default]
    Center,
    Left,
    Right,
}

impl FigureSide {
    /// 获取立绘 ID
    pub fn get_id(&self) -> &'static str {
        match self {
            Self::Center => "fig-center",
            Self::Left => "fig-left",
            Self::Right => "fig-right",
        }
    }
}

/// 立绘引用
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FigureId {
    Id(String),
    Side(FigureSide),
}

impl FigureId {
    pub fn is_id(&self) -> bool {
        matches!(self, Self::Id(_))
    }

    pub fn is_side(&self) -> bool {
        matches!(self, Self::Side(_))
    }

    /// 获取立绘 ID
    pub fn get_id(&self) -> &str {
        match self {
            Self::Id(id) => id,
            Self::Side(side) => side.get_id(),
        }
    }
}

impl<S: AsRef<str>> From<S> for FigureId {
    fn from(value: S) -> Self {
        match value.as_ref() {
            "fig-center" => Self::Side(FigureSide::Center),
            "fig-left" => Self::Side(FigureSide::Left),
            "fig-right" => Self::Side(FigureSide::Right),
            id => Self::Id(id.to_string()),
        }
    }
}

impl_from_str_for_from!(FigureId);

// -------- 样式 --------

/// 字体大小
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum FontSize {
    #[default]
    Default,
    Small,
    Medium,
    Large,
}

/// 通用颜色表示
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, From, Into)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: f32,
}

impl Color {
    pub fn from_str_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let parse = |range| u8::from_str_radix(&s[range], 16).ok();
        let red = parse(0..2)?;
        let green = parse(2..4)?;
        let blue = parse(4..6)?;
        Some(Self {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    pub fn from_str_rgb(s: &str) -> Option<Self> {
        let s = s.trim().strip_prefix("rgb(")?.strip_suffix(")")?;
        let mut parts = s.split(',').map(str::trim);
        let mut parse = || parts.next()?.parse().ok();
        let red = parse()?;
        let green = parse()?;
        let blue = parse()?;
        parts.next().is_none().then_some(Self {
            red,
            green,
            blue,
            alpha: 1.0,
        })
    }

    pub fn from_str_rgba(s: &str) -> Option<Self> {
        let s = s.trim().strip_prefix("rgba(")?.strip_suffix(")")?;
        let mut parts = s.split(',').map(str::trim);
        let mut parse = || parts.next()?.parse().ok();
        let red = parse()?;
        let green = parse()?;
        let blue = parse()?;
        let alpha = parts.next()?.parse().ok()?;
        parts.next().is_none().then_some(Self {
            red,
            green,
            blue,
            alpha,
        })
    }

    pub fn fmt_hex_upper<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        let Self {
            red, green, blue, ..
        } = self;
        write!(f, "#{red:02X}{green:02X}{blue:02X}")
    }

    pub fn fmt_hex_lower<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        let Self {
            red, green, blue, ..
        } = self;
        write!(f, "#{red:02x}{green:02x}{blue:02x}")
    }

    pub fn fmt_css_rgb<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        let Self {
            red, green, blue, ..
        } = self;
        write!(f, "rgb({red},{green},{blue})")
    }

    pub fn fmt_css_rgba<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        let Self {
            red,
            green,
            blue,
            alpha,
        } = self;
        write!(f, "rgba({red},{green},{blue},{alpha})")
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 1.,
        }
    }
}

impl FromStr for Color {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_hex(s)
            .or_else(|| Self::from_str_rgb(s))
            .or_else(|| Self::from_str_rgba(s))
            .ok_or(concat!(
                "`Color` 应为 `#RRGGBB` / `rgb(R,G,B)` / `rgba(R,G,B,A)` 的格式, ",
                "颜色在 [0..256) 间, 透明度在 [0,1] 间"
            ))
    }
}

/// Live2D 立绘扩展显示区域
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
pub struct Live2dBounds {
    pub west: isize,
    pub north: isize,
    pub east: isize,
    pub south: isize,
}

impl FromStr for Live2dBounds {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        (|| {
            let mut parts = s.split(',');
            let mut parse = || parts.next()?.trim().parse().ok();
            let west = parse()?;
            let north = parse()?;
            let east = parse()?;
            let south = parse()?;
            parts.next().is_none().then_some(Self {
                west,
                north,
                east,
                south,
            })
        })()
        .ok_or("`Live2dBounds` 应为 `isize,isize,isize,isize` 的格式")
    }
}

impl fmt::Display for Live2dBounds {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Live2dBounds {
            west,
            north,
            east,
            south,
        } = self;
        write!(f, "{west},{north},{east},{south}")
    }
}

/// Live2D 立绘眨眼参数
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Live2dBlink {
    pub blink_interval: Option<usize>,
    pub blink_interval_random: Option<usize>,
    pub closing_duration: Option<usize>,
    pub closed_duration: Option<usize>,
    pub opening_duration: Option<usize>,
}

impl_from_str_for_serde_json!(Live2dBlink);
impl_display_for_serde_json!(Live2dBlink);

/// Live2D 立绘注视参数
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Live2dFocus {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub instant: Option<bool>,
}

impl_from_str_for_serde_json!(Live2dFocus);
impl_display_for_serde_json!(Live2dFocus);

// -------- 转场 --------

/// 缓动类型
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
    EnumString,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum Ease {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
    #[strum(serialize = "circIn")]
    CircleIn,
    #[strum(serialize = "circOut")]
    CircleOut,
    #[strum(serialize = "circInOut")]
    CircleInOut,
    BackIn,
    BackOut,
    BackInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
    Anticipate,
}

/// 全屏文字动画效果
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum IntroAnimation {
    #[default]
    FadeIn,
    SlideIn,
    TypingEffect,
    PixelateEffect,
    RevealAnimation,
}

// -------- 动画 --------

/// 变换效果
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    // 基础变换
    pub position: Option<Position>,
    pub rotation: Option<f32>,
    pub scale: Option<Scale>,
    // 基础效果
    pub alpha: Option<f32>,
    pub blur: Option<usize>,
    // 颜色调整滤镜
    pub brightness: Option<f32>,
    pub contrast: Option<f32>,
    pub saturation: Option<f32>,
    pub gamma: Option<f32>,
    pub color_red: Option<u8>,
    pub color_green: Option<u8>,
    pub color_blue: Option<u8>,
    // 泛光滤镜
    pub bloom: Option<f32>,
    pub bloom_brightness: Option<f32>,
    pub bloom_blur: Option<usize>,
    pub bloom_threshold: Option<f32>,
    // 倒角滤镜
    pub bevel: Option<f32>,
    pub bevel_thickness: Option<usize>,
    pub bevel_rotation: Option<f32>,
    pub bevel_red: Option<u8>,
    pub bevel_green: Option<u8>,
    pub bevel_blue: Option<u8>,
    // 其他滤镜
    #[serde_as(as = "Option<BoolFromInt>")]
    pub old_film: Option<bool>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub dot_film: Option<bool>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub rgb_film: Option<bool>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub glitch_film: Option<bool>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub godray_film: Option<bool>,
    #[serde_as(as = "Option<BoolFromInt>")]
    pub reflection_film: Option<bool>,
    pub shockwave: Option<f32>,
    pub radius_alpha: Option<f32>,
}

impl Transform {
    pub fn validate(&self) -> anyhow::Result<()> {
        try_validate(|errors| {
            if let Some(scale) = &self.scale
                && let Err(error) = scale.validate()
            {
                errors.push(anyhow::anyhow!("`scale` 参数: {error}"));
            }
            if let Some(alpha) = self.alpha
                && !(0. ..=1.).contains(&alpha)
            {
                errors.push(anyhow::anyhow!("`alpha` 参数出界, 其范围是 [0, 1]"));
            }
            if let Some(brightness) = self.brightness
                && brightness < 0.
            {
                errors.push(anyhow::anyhow!("`brightness` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(contrast) = self.contrast
                && contrast < 0.
            {
                errors.push(anyhow::anyhow!("`contrast` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(saturation) = self.saturation
                && saturation < 0.
            {
                errors.push(anyhow::anyhow!("`saturation` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(gamma) = self.gamma
                && gamma < 0.
            {
                errors.push(anyhow::anyhow!("`gamma` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(bloom) = self.bloom
                && bloom < 0.
            {
                errors.push(anyhow::anyhow!("`bloom` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(bloom_brightness) = self.bloom_brightness
                && bloom_brightness < 0.
            {
                errors.push(anyhow::anyhow!(
                    "`bloom_brightness` 参数出界, 其范围是 [0, +inf)"
                ));
            }
            if let Some(bloom_threshold) = self.bloom_threshold
                && !(0. ..=1.).contains(&bloom_threshold)
            {
                errors.push(anyhow::anyhow!(
                    "`bloom_threshold` 参数出界, 其范围是 [0, 1]"
                ));
            }
            if let Some(bevel) = self.bevel
                && bevel < 0.
            {
                errors.push(anyhow::anyhow!("`bevel` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(radius_alpha) = self.radius_alpha
                && !(0. ..=1.).contains(&radius_alpha)
            {
                errors.push(anyhow::anyhow!("`radius_alpha` 参数出界, 其范围是 [0, 1]"));
            }
        })
    }
}

impl_from_str_for_serde_json!(Transform);
impl_display_for_serde_json!(Transform);

#[serde_as]
#[skip_serializing_none]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Position {
    pub x: Option<isize>,
    pub y: Option<isize>,
}

impl_from_str_for_serde_json!(Position);
impl_display_for_serde_json!(Position);

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Scale {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Scale {
    pub fn validate(&self) -> anyhow::Result<()> {
        try_validate(|errors| {
            if let Some(x) = self.x
                && x < 0.
            {
                errors.push(anyhow::anyhow!("`x` 参数出界, 其范围是 [0, +inf)"));
            }
            if let Some(y) = self.y
                && y < 0.
            {
                errors.push(anyhow::anyhow!("`y` 参数出界, 其范围是 [0, +inf)"));
            }
        })
    }
}

impl_from_str_for_serde_json!(Scale);
impl_display_for_serde_json!(Scale);

/// 动画片段 (继承自变换效果)
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Animation {
    pub duration: usize,
    pub ease: Option<Ease>,
    #[serde(flatten, default)]
    pub transform: Transform,
}

impl Animation {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.transform.validate()
    }
}

impl_from_str_for_serde_json!(Animation);
impl_display_for_serde_json!(Animation);

/// 多段动画
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    PartialOrd,
    From,
    Into,
    Deref,
    DerefMut,
    Serialize,
    Deserialize,
)]
pub struct AnimationList(Vec<Animation>);

impl AnimationList {
    pub fn validate(&self) -> anyhow::Result<()> {
        try_validate(|errors| {
            errors.extend(self.iter().enumerate().filter_map(|(i, animation)| {
                animation
                    .validate()
                    .err()
                    .map(|error| anyhow::anyhow!("第 {i} 个动画: {error}"))
            }));
        })
    }
}

impl_from_str_for_serde_json!(AnimationList);
impl_display_for_serde_json!(AnimationList);

// -------- 执行 --------

/// 执行时序
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum Forward {
    #[default]
    Wait,
    Continue,
    Next,
}

/// 持续模式
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString,
)]
#[strum(serialize_all = "camelCase")]
pub enum Sustain {
    #[default]
    Wait,
    Keep,
    Parallel,
}

// #[cfg(feature = "lsp")]
// pub use lsp_ext::*;

#[cfg(feature = "lsp")]
mod lsp_ext {
    use json_complete::{ToJsonSchema, Value, json};

    use super::*;

    impl ToJsonSchema for Live2dBlink {
        fn schema() -> Value {
            json! {{
                "blinkInterval":       number "眨眼间隔 (ms)",
                "blinkIntervalRandom": number "眨眼间隔随机值 (ms)",
                "closingDuration":     number "闭眼持续时间 (ms)",
                "closedDuration":      number "闭眼时间 (ms)",
                "openingDuration":     number "睁眼持续时间 (ms)",
            }}
        }
    }

    impl ToJsonSchema for Live2dFocus {
        fn schema() -> Value {
            json! {{
                "x":       number "注视点 x 轴坐标",
                "y":       number "注视点 y 轴坐标",
                "instant": bool   "立即注视",
            }}
        }
    }

    impl ToJsonSchema for Transform {
        fn schema() -> Value {
            json! {{
                // 基础变换
                "position": (Position::schema()) "位置",
                "rotation": number               "旋转 (rad)",
                "scale":    (Scale::schema())    "缩放",
                // 基础效果
                "alpha": number "透明度 [0, 1]",
                "blur":  number "模糊 [0..+inf)",
                // 颜色调整滤镜
                "brightness": number "亮度 [0, +inf)",
                "contrast":   number "对比度 [0, +inf)",
                "saturation": number "饱和度 [0, +inf)",
                "gamma":      number "伽马值 [0, +inf)",
                "colorRed":   number "色调红色分量 [0..256)",
                "colorGreen": number "色调绿色分量 [0..256)",
                "colorBlue":  number "色调蓝色分量 [0..256)",
                // 泛光滤镜
                "bloom":           number "泛光强度 [0..+inf)",
                "bloomBrightness": number "泛光亮度 [0, +inf)",
                "bloomBlur":       number "泛光模糊 [0..+inf)",
                "bloomThreshold":  number "泛光阈值 [0, 1]",
                // 倒角滤镜
                "bevel":          number "倒角透明度 [0, 1]",
                "bevelThickness": number "倒角厚度 [0..+inf)",
                "bevelRotation":  number "倒角旋转 (rad)",
                "bevelRed":       number "倒角红色分量 [0..256)",
                "bevelGreen":     number "倒角绿色分量 [0..256)",
                "bevelBlue":      number "倒角蓝色分量 [0..256)",
                // 其他滤镜
                "oldFilm":        number "老电影滤镜 {0, 1}",
                "dotFilm":        number "点状滤镜 {0, 1}",
                "rgbFilm":        number "RGB 分离滤镜 {0, 1}",
                "glitchFilm":     number "故障滤镜 {0, 1}",
                "godrayFilm":     number "光辉滤镜 {0, 1}",
                "reflectionFilm": number "反射滤镜 {0, 1}",
                "shockwave":      number "冲击波相位",
                "radiusAlpha":    number "径向渐变透明半径 [0, +inf)",
            }}
        }
    }

    impl ToJsonSchema for Position {
        fn schema() -> Value {
            json! {{
                "x": number "x 轴坐标 (pix)",
                "y": number "y 轴坐标 (pix)",
            }}
        }
    }

    impl ToJsonSchema for Scale {
        fn schema() -> Value {
            json! {{
                "x": number "x 轴缩放 [0, +inf)",
                "y": number "y 轴缩放 [0, +inf)",
            }}
        }
    }

    impl ToJsonSchema for Animation {
        fn schema() -> Value {
            json! {{
                "duration": number "持续时间 (ms)",
                "ease":     string "缓动类型",
            }}
            .inherit(&Transform::schema())
        }
    }

    impl ToJsonSchema for AnimationList {
        fn schema() -> Value {
            json! {[ (Animation::schema()) ]}
        }
    }
}

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    #[test]
    fn color_roundtrip_hex() {
        let original = "#FFAABB";
        let color = Color::from_str(original).unwrap();
        let mut output = String::new();
        color.fmt_hex_upper(&mut output).unwrap();
        assert_eq!(output, original.to_uppercase());
    }

    #[test]
    fn color_roundtrip_rgb() {
        let original = "rgb(12,34,56)";
        let color = Color::from_str(original).unwrap();
        let mut output = String::new();
        color.fmt_css_rgb(&mut output).unwrap();
        assert_eq!(output, original);
    }

    #[test]
    fn color_roundtrip_rgba() {
        let original = "rgba(10,20,30,0.75)";
        let color = Color::from_str(original).unwrap();
        let mut output = String::new();
        color.fmt_css_rgba(&mut output).unwrap();
        assert_eq!(output, original);
    }

    #[test]
    fn live2d_bounds_roundtrip() {
        let original = "10,-20,30,-40";
        let bounds = Live2dBounds::from_str(original).unwrap();
        let output = bounds.to_string();
        assert_eq!(output, original);
    }

    #[test]
    fn ease_parsing() {
        assert_eq!(Ease::from_str("linear").unwrap(), Ease::Linear);
        assert_eq!(Ease::from_str("circIn").unwrap(), Ease::CircleIn);
        assert!(Ease::from_str("unknown").is_err());
    }

    #[test]
    fn object_id_roundtrip() {
        let cases = vec!["stage-main", "bg-main", "fig-center", "custom-id"];
        for id in cases {
            let obj = ObjectId::from(id);
            let output = obj.to_string();
            let parsed = ObjectId::from_str(&output).unwrap();
            assert_eq!(parsed, obj);
            assert_eq!(output, id);
        }
    }

    #[test]
    fn transform_partial_roundtrip() {
        let transform = Transform {
            alpha: Some(0.5),
            brightness: Some(1.2),
            scale: Some(Scale {
                x: Some(1.5),
                y: None,
            }),
            ..Default::default()
        };
        let json = transform.to_string();
        let parsed = Transform::from_str(&json).unwrap();
        assert_eq!(parsed.alpha, transform.alpha);
        assert_eq!(parsed.brightness, transform.brightness);
        assert_eq!(
            parsed.scale.as_ref().unwrap().x,
            transform.scale.as_ref().unwrap().x
        );
        assert_eq!(parsed.scale.as_ref().unwrap().y, None);
        // 其他字段应均为 None
        assert!(parsed.position.is_none());
        assert!(parsed.rotation.is_none());
        assert!(parsed.blur.is_none());
    }

    #[test]
    fn transform_validate_ok() {
        let t = Transform {
            alpha: Some(0.5),
            brightness: Some(1.2),
            scale: Some(Scale {
                x: Some(1.5),
                y: Some(0.0),
            }),
            ..Default::default()
        };
        t.validate().unwrap();
    }

    #[test]
    fn transform_validate_errors() {
        let t = Transform {
            alpha: Some(1.5),
            brightness: Some(-0.1),
            scale: Some(Scale {
                x: Some(-2.0),
                y: Some(1.0),
            }),
            bevel: Some(-5.0),
            bloom_threshold: Some(1.2),
            radius_alpha: Some(-0.1),
            ..Default::default()
        };
        let err = t.validate().unwrap_err();
        let msg = err.to_string();
        // 检查错误消息中是否包含各个关键字段
        assert!(msg.contains("alpha"), "缺少 alpha 错误");
        assert!(msg.contains("brightness"), "缺少 brightness 错误");
        assert!(msg.contains("x"), "缺少 scale/x 错误");
        assert!(msg.contains("bevel"), "缺少 bevel 错误");
        assert!(msg.contains("bloom_threshold"), "缺少 bloom_threshold 错误");
        assert!(msg.contains("radius_alpha"), "缺少 radius_alpha 错误");
    }

    #[test]
    fn animation_list_roundtrip() {
        let list = AnimationList(vec![
            Animation {
                duration: 100,
                ease: Some(Ease::Linear),
                transform: Transform {
                    alpha: Some(0.5),
                    ..Default::default()
                },
            },
            Animation {
                duration: 200,
                ease: None,
                transform: Transform::default(),
            },
        ]);
        let json = list.to_string();
        let parsed = AnimationList::from_str(&json).unwrap();
        assert_eq!(parsed.len(), list.len());
        assert_eq!(parsed[0].duration, list[0].duration);
        assert_eq!(parsed[0].ease, list[0].ease);
        assert_eq!(parsed[0].transform.alpha, list[0].transform.alpha);
        assert_eq!(parsed[1].duration, list[1].duration);
        assert!(parsed[1].transform.alpha.is_none());
    }

    #[test]
    fn forward_ord() {
        assert!(Forward::Next > Forward::Continue);
        assert!(Forward::Continue > Forward::Wait);
        assert_eq!(Forward::Wait.max(Forward::Next), Forward::Next);
    }
}
