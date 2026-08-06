//! 舞台状态

use std::{collections::HashMap, rc::Rc};

use webgal_language_core::element::Transform;

/// 舞台状态
#[derive(Debug, Clone, Default)]
pub struct Stage {
    // 对话
    pub textbox: Rc<Textbox>,
    // 舞台对象
    pub background: Option<Rc<Background>>,
    pub figures: HashMap<String, Rc<Figure>>,
    pub bgm: Option<Rc<String>>,
    pub sounds: HashMap<String, Rc<String>>,
    // 舞台效果
    pub transform: Option<Rc<Transform>>,
    pub pixi: Option<Rc<String>>,
}

/// 对话框
#[derive(Debug, Clone, Default)]
pub struct Textbox {
    pub speaker: String,
    pub content: Vec<String>, // 处理后纯文本
    pub show: bool,
}

/// 舞台背景
#[derive(Debug, Clone, Default)]
pub struct Background {
    pub path: String,
    pub transform: Transform,
    pub exit: Option<String>,
}

/// 立绘
#[derive(Debug, Clone, Default)]
pub struct Figure {
    pub path: String,
    // 效果
    pub transform: Transform,
    pub exit: Option<String>,
    // Live2D / Spine 立绘
    pub motion: Option<String>,
    pub expression: Option<String>,
}
