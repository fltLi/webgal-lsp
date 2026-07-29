//! 舞台状态

use std::{collections::HashMap, rc::Rc};

use webgal_language_core::element::{AnimationList, FigureSide, Live2dFocus, Transform};

/// 舞台状态
#[derive(Debug, Clone, Default)]
pub struct Stage {
    // 舞台对象
    pub background: Option<Rc<Background>>,
    pub figures: HashMap<String, Rc<Figure>>,
    pub bgm: Option<Rc<String>>,
    pub sounds: HashMap<String, Rc<String>>,
    // 舞台效果
    pub transform: Option<Rc<Transform>>,
    pub pixi: Option<Rc<String>>,
}

/// 舞台背景
#[derive(Debug, Clone, Default)]
pub struct Background {
    pub path: String,
    pub transform: Transform,
    pub exit: Option<AnimationList>,
    pub complex_animation: Vec<String>,
}

/// 立绘
#[derive(Debug, Clone, Default)]
pub struct Figure {
    pub path: String,
    // 效果
    pub side: FigureSide,
    pub transform: Transform,
    pub exit: Option<AnimationList>,
    pub complex_animation: Vec<String>,
    // 图像立绘
    pub mouth_open: Option<String>,
    pub mouth_half_open: Option<String>,
    pub mouth_close: Option<String>,
    pub eyes_open: Option<String>,
    pub eyes_close: Option<String>,
    // Live2D / Spine 立绘
    pub skin: Option<String>,
    pub motion: Option<String>,
    pub expression: Option<String>,
    pub focus: Option<Live2dFocus>,
}
