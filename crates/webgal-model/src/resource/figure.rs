//! 立绘资源

use derive_more::{From, TryInto};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, Map, serde_as};

pub use crate::element::Live2dBounds;
use crate::{impl_display_for_serde_json, impl_from_str_for_serde_json};

/// 立绘资源枚举
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, From, TryInto)]
pub enum Figure {
    #[default]
    Image,
    Spine, // 暂不支持
    // Live2D
    Live2d(Live2dModel),
    Wmdl(WmdlModel),
    Composite, // 暂不支持
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FigureKind {
    #[default]
    Image,
    Spine,
    Live2d,
    Wmdl,
    Composite,
}

impl Figure {
    pub fn get_type(&self) -> FigureKind {
        match self {
            Self::Image => FigureKind::Image,
            Self::Spine => FigureKind::Spine,
            Self::Live2d(_) => FigureKind::Live2d,
            Self::Wmdl(_) => FigureKind::Wmdl,
            Self::Composite => FigureKind::Composite,
        }
    }
}

// -------- Live2D --------

/// Live2D 立绘模型
#[serde_as]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Live2dModel {
    #[serde(default)]
    pub version: String,
    // 模型
    pub model: String,
    pub physics: String,
    pub textures: Vec<String>,
    #[serde_as(as = "Map<_, _>")]
    #[serde(default)]
    pub motions: Vec<(String, Vec<Live2dMotion>)>,
    #[serde(default)]
    pub expressions: Vec<Live2dExpression>,
    // 渲染
    #[serde(default)]
    pub layout: Live2dLayout,
    #[serde(rename = "hit_areas_custom", default)]
    pub hit_areas: HitAreas,
}

impl_from_str_for_serde_json!(Live2dModel);
impl_display_for_serde_json!(Live2dModel);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Live2dLayout {
    #[serde(rename = "center_x")]
    pub x: isize,
    #[serde(rename = "center_y")]
    pub y: isize,
    pub width: usize,
}

impl Default for Live2dLayout {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 2,
        }
    }
}

impl_from_str_for_serde_json!(Live2dLayout);
impl_display_for_serde_json!(Live2dLayout);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(default)]
pub struct HitAreas {
    pub head_x: (f32, f32),
    pub head_y: (f32, f32),
    pub body_x: (f32, f32),
    pub body_y: (f32, f32),
}

impl Default for HitAreas {
    fn default() -> Self {
        Self {
            head_x: (-0.25, 1.),
            head_y: (0.25, 0.2),
            body_x: (-0.3, 0.2),
            body_y: (0.3, -1.9),
        }
    }
}

impl_from_str_for_serde_json!(HitAreas);
impl_display_for_serde_json!(HitAreas);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Live2dMotion {
    pub file: String,
}

impl_from_str_for_serde_json!(Live2dMotion);
impl_display_for_serde_json!(Live2dMotion);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Live2dExpression {
    pub name: String,
    pub file: String,
}

impl_from_str_for_serde_json!(Live2dExpression);
impl_display_for_serde_json!(Live2dExpression);

// -------- WMDL --------

/// Live2D 拼好模
#[serde_as]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WmdlModel {
    // 模型
    #[serde(default)]
    pub name: String,
    #[serde(rename = "modelRelativePath")]
    pub model: String,
    #[serde(default)]
    pub sub_models: Vec<WmdlSubModel>,
    // 语句
    pub figure_template: String,
    pub transform_template: String,
    // 渲染
    #[serde(default)]
    pub x: isize,
    #[serde(default)]
    pub y: isize,
    #[serde(default)]
    pub scale: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub reverse_x: bool,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    pub bounds: Live2dBounds,
}

impl_from_str_for_serde_json!(WmdlModel);
impl_display_for_serde_json!(WmdlModel);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WmdlSubModel {
    #[serde(rename = "modelRelativePath")]
    pub model: String,
    // 渲染
    #[serde(default)]
    pub offset_x: isize,
    #[serde(default)]
    pub offset_y: isize,
}

impl_from_str_for_serde_json!(WmdlSubModel);
impl_display_for_serde_json!(WmdlSubModel);

// -------- lsp --------

#[cfg(feature = "lsp")]
pub use lsp_ext::*;

#[cfg(feature = "lsp")]
mod lsp_ext {
    use path_tree::Folder;

    use super::*;

    impl FigureKind {
        /// 依据路径识别模型类型
        ///
        /// # Returns
        /// 模型类型及其路径 (例如去掉部分模型的 `?type=...` 标识).
        pub fn from_path(model: &str) -> (Self, &str) {
            if let Some(model) = model.strip_suffix("?type=spine") {
                return (Self::Spine, model);
            }
            let kind = [
                (".skel", Self::Spine),
                (".json", Self::Live2d),
                (".wmdl", Self::Wmdl),
                (".jsonl", Self::Composite),
            ]
            .iter()
            .find_map(|&(extension, kind)| model.ends_with(extension).then_some(kind))
            .unwrap_or(Self::Image);
            (kind, model)
        }
    }

    // -------- motion --------

    /// 立绘模型立绘 / 表情调用信息
    #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum FigureInfo {
        #[default]
        Image,
        Spine,
        Live2d {
            motions: Folder<()>,
            expressions: Folder<()>,
        },
        Wmdl {
            import: String,
        },
        Composite,
    }

    impl FigureInfo {
        pub fn from_figure(model: &Figure) -> Self {
            match model {
                Figure::Image => Self::Image,
                Figure::Spine => Self::Spine,
                Figure::Live2d(model) => Self::from_live2d(model),
                Figure::Wmdl(model) => Self::from_wmdl(model),
                Figure::Composite => Self::Composite,
            }
        }

        pub fn from_live2d(model: &Live2dModel) -> Self {
            let motions = model
                .motions
                .iter()
                .map(|(motion, _)| (motion, ()))
                .collect();
            let expressions = model
                .expressions
                .iter()
                .map(|Live2dExpression { name, .. }| (name, ()))
                .collect();
            Self::Live2d {
                motions,
                expressions,
            }
        }

        pub fn from_wmdl(model: &WmdlModel) -> Self {
            Self::Wmdl {
                import: model.model.clone(),
            }
        }

        pub fn try_from_type(kind: &FigureKind) -> Option<Self> {
            match kind {
                FigureKind::Image => Some(Self::Image),
                FigureKind::Spine => Some(Self::Spine),
                FigureKind::Composite => Some(Self::Composite),
                _ => None,
            }
        }

        pub fn get_type(&self) -> FigureKind {
            match self {
                Self::Image => FigureKind::Image,
                Self::Spine => FigureKind::Spine,
                Self::Live2d { .. } => FigureKind::Live2d,
                Self::Wmdl { .. } => FigureKind::Wmdl,
                Self::Composite => FigureKind::Composite,
            }
        }
    }

    impl Figure {
        pub fn to_info(&self) -> FigureInfo {
            FigureInfo::from_figure(self)
        }
    }

    impl Live2dModel {
        pub fn to_info(&self) -> FigureInfo {
            FigureInfo::from_live2d(self)
        }
    }

    impl WmdlModel {
        pub fn to_info(&self) -> FigureInfo {
            FigureInfo::from_wmdl(self)
        }
    }

    impl FigureKind {
        pub fn try_to_info(&self) -> Option<FigureInfo> {
            FigureInfo::try_from_type(self)
        }
    }
}
