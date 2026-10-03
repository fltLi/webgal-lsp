//! 立绘资源

use derive_more::{From, TryInto};
use itertools::Either;
use path_tree::Folder;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;
use serde_with::{DisplayFromStr, Map, serde_as};

use crate::{impl_display_for_serde_json, impl_from_str_for_serde_json};

pub use crate::element::Live2dBounds;

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

    pub fn to_info(&self) -> FigureInfo {
        FigureInfo::from_figure(self)
    }
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

    pub fn try_to_info(&self) -> Option<FigureInfo> {
        FigureInfo::try_from_type(self)
    }
}

/// 立绘模型立绘 / 表情调用信息
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FigureInfo {
    #[default]
    Image,
    Spine,
    Live2d {
        kind: Live2dModelKind,
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
        match model {
            Live2dModel::Cubism2(model) => Self::from_live2d2(model),
            Live2dModel::Cubism3(model) => Self::from_live2d3(model),
        }
    }

    pub fn from_live2d2(model: &Live2dModel2) -> Self {
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
            kind: Live2dModelKind::Cubism2,
            motions,
            expressions,
        }
    }

    pub fn from_live2d3(model: &Live2dModel3) -> Self {
        let motions = model
            .assets
            .motions
            .iter()
            .map(|(motion, _)| (motion, ()))
            .collect();
        let expressions = model
            .assets
            .expressions
            .iter()
            .map(|Live2dExpression { name, .. }| (name, ()))
            .collect();
        Self::Live2d {
            kind: Live2dModelKind::Cubism3,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Live2dModelKind {
    Cubism2,
    Cubism3,
}

// -------- Live2D --------

/// Live2D 立绘模型
#[derive(Debug, Clone, PartialEq, PartialOrd, From, TryInto)]
pub enum Live2dModel {
    Cubism2(Live2dModel2),
    Cubism3(Live2dModel3),
}

impl Live2dModel {
    pub fn get_type(&self) -> Live2dModelKind {
        match self {
            Self::Cubism2(_) => Live2dModelKind::Cubism2,
            Self::Cubism3(_) => Live2dModelKind::Cubism3,
        }
    }

    pub fn model(&self) -> &str {
        match self {
            Self::Cubism2(model) => &model.model,
            Self::Cubism3(model) => &model.assets.model,
        }
    }

    pub fn physics(&self) -> Option<&str> {
        match self {
            Self::Cubism2(model) => model.physics.as_deref(),
            Self::Cubism3(model) => model.assets.physics.as_deref(),
        }
    }

    pub fn textures(&self) -> &[String] {
        match self {
            Self::Cubism2(model) => &model.textures,
            Self::Cubism3(model) => &model.assets.textures,
        }
    }

    pub fn motions(&self) -> &[(String, Vec<Live2dMotion>)] {
        match self {
            Self::Cubism2(model) => &model.motions,
            Self::Cubism3(model) => &model.assets.motions,
        }
    }

    pub fn expressions(&self) -> &[Live2dExpression] {
        match self {
            Self::Cubism2(model) => &model.expressions,
            Self::Cubism3(model) => &model.assets.expressions,
        }
    }

    pub fn to_info(&self) -> FigureInfo {
        FigureInfo::from_live2d(self)
    }

    pub fn resources(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::Cubism2(model) => Either::Left(model.resources()),
            Self::Cubism3(model) => Either::Right(model.resources()),
        }
    }

    /// 依据 `Version` / `version` 的**取值**判定模型描述的方言
    ///
    /// # Behavior
    /// * `"Version": 3` - Cubism3 / Cubism4 导出的 `*.model3.json`;
    /// * `"version": "Sample 1.0.0"` - Cubism2 的模型描述;
    /// * 其余取值一律返回 `None`.
    fn kind_of(value: &Value) -> Option<Live2dModelKind> {
        let object = value.as_object()?;

        if object.get("Version").and_then(Value::as_u64) == Some(3) {
            return Some(Live2dModelKind::Cubism3);
        }
        if object.get("version").and_then(Value::as_str) == Some("Sample 1.0.0") {
            return Some(Live2dModelKind::Cubism2);
        }

        None
    }
}

impl Serialize for Live2dModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Cubism2(model) => model.serialize(serializer),
            Self::Cubism3(model) => model.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Live2dModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // 先整块读成 JSON, 以便在解析具体结构之前先看方言标记
        let value = Value::deserialize(deserializer)?;

        match Self::kind_of(&value) {
            Some(Live2dModelKind::Cubism2) => serde_json::from_value(value)
                .map(Self::Cubism2)
                .map_err(de::Error::custom),
            Some(Live2dModelKind::Cubism3) => serde_json::from_value(value)
                .map(Self::Cubism3)
                .map_err(de::Error::custom),
            None => Err(de::Error::custom(
                "无法识别 Live2D 模型描述: 需要 `\"Version\": 3` 或 `\"version\": \"Sample 1.0.0\"`",
            )),
        }
    }
}

impl_from_str_for_serde_json!(Live2dModel);
impl_display_for_serde_json!(Live2dModel);

// -------- Live2D Cubism2 --------

/// Live2D Cubism2 立绘模型
#[serde_as]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Live2dModel2 {
    #[serde(default)]
    pub version: String,
    // 模型
    pub model: String,
    #[serde(default)]
    pub physics: Option<String>,
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

impl_from_str_for_serde_json!(Live2dModel2);
impl_display_for_serde_json!(Live2dModel2);

impl Live2dModel2 {
    pub fn to_info(&self) -> FigureInfo {
        FigureInfo::from_live2d2(self)
    }

    pub fn resources(&self) -> impl Iterator<Item = &str> {
        [&self.model]
            .into_iter()
            .chain(self.physics.iter())
            .chain(self.textures.iter())
            .chain(
                self.motions
                    .iter()
                    .flat_map(|(_, motions)| motions.iter().map(|motion| &motion.file)),
            )
            .chain(self.expressions.iter().map(|expression| &expression.file))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Live2dLayout {
    #[serde(rename = "center_x")]
    pub x: i32,
    #[serde(rename = "center_y")]
    pub y: i32,
    pub width: u32,
}

impl_from_str_for_serde_json!(Live2dLayout);
impl_display_for_serde_json!(Live2dLayout);

impl Default for Live2dLayout {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(default)]
pub struct HitAreas {
    pub head_x: (f32, f32),
    pub head_y: (f32, f32),
    pub body_x: (f32, f32),
    pub body_y: (f32, f32),
}

impl_from_str_for_serde_json!(HitAreas);
impl_display_for_serde_json!(HitAreas);

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Live2dMotion {
    #[serde(alias = "File")]
    pub file: String,
}

impl_from_str_for_serde_json!(Live2dMotion);
impl_display_for_serde_json!(Live2dMotion);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Live2dExpression {
    #[serde(alias = "Name")]
    pub name: String,
    #[serde(alias = "File")]
    pub file: String,
}

impl_from_str_for_serde_json!(Live2dExpression);
impl_display_for_serde_json!(Live2dExpression);

// -------- Live2D Cubism3 --------

/// Live2D Cubism3 立绘模型
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Live2dModel3 {
    #[serde(default)]
    pub version: u8,
    #[serde(default)]
    pub name: String,
    // 模型
    #[serde(rename = "FileReferences")]
    pub assets: Live2dAssets,
    #[serde(default)]
    pub groups: Vec<Live2dGroup>,
}

impl_from_str_for_serde_json!(Live2dModel3);
impl_display_for_serde_json!(Live2dModel3);

impl Live2dModel3 {
    pub fn to_info(&self) -> FigureInfo {
        FigureInfo::from_live2d3(self)
    }

    pub fn resources(&self) -> impl Iterator<Item = &str> {
        self.assets.resources()
    }
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Live2dAssets {
    #[serde(rename = "Moc")]
    pub model: String,
    #[serde(default)]
    pub physics: Option<String>,
    pub textures: Vec<String>,
    #[serde_as(as = "Map<_, _>")]
    #[serde(default)]
    pub motions: Vec<(String, Vec<Live2dMotion>)>,
    #[serde(default)]
    pub expressions: Vec<Live2dExpression>,
    // 渲染
    #[serde(default)]
    pub pose: Option<String>,
    #[serde(rename = "DisplayInfo", default)]
    pub display: Option<String>,
}

impl_from_str_for_serde_json!(Live2dAssets);
impl_display_for_serde_json!(Live2dAssets);

impl Live2dAssets {
    pub fn resources(&self) -> impl Iterator<Item = &str> {
        [&self.model]
            .into_iter()
            .chain(self.physics.iter())
            .chain(self.textures.iter())
            .chain(self.pose.iter())
            .chain(self.display.iter())
            .chain(
                self.motions
                    .iter()
                    .flat_map(|(_, motions)| motions.iter().map(|motion| &motion.file)),
            )
            .chain(self.expressions.iter().map(|expression| &expression.file))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Live2dGroup {
    pub target: String,
    pub name: String,
    pub ids: Vec<String>,
}

impl_from_str_for_serde_json!(Live2dGroup);
impl_display_for_serde_json!(Live2dGroup);

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
    pub x: i32,
    #[serde(default)]
    pub y: i32,
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

impl WmdlModel {
    pub fn to_info(&self) -> FigureInfo {
        FigureInfo::from_wmdl(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WmdlSubModel {
    #[serde(rename = "modelRelativePath")]
    pub model: String,
    // 渲染
    #[serde(default)]
    pub offset_x: i32,
    #[serde(default)]
    pub offset_y: i32,
}

impl_from_str_for_serde_json!(WmdlSubModel);
impl_display_for_serde_json!(WmdlSubModel);

#[cfg(test)]
mod tests {
    // This module is generated by AI.

    use super::*;

    /// Cubism2 模型描述 (取自官方示例模型 Bestdori 打包版)
    const CUBISM2: &str = r#"{
        "version": "Sample 1.0.0",
        "layout": { "center_x": 0, "center_y": 0, "width": 2 },
        "hit_areas_custom": {
            "head_x": [-0.25, 1], "head_y": [0.25, 0.2],
            "body_x": [-0.3, 0.2], "body_y": [0.3, -1.9]
        },
        "model": "data/model.moc",
        "physics": "data/physics.json",
        "textures": ["data/textures/texture_00.png"],
        "motions": { "idle01": [{ "file": "data/motions/idle01.mtn" }] },
        "expressions": [{ "name": "idle01", "file": "data/expressions/idle01.exp.json" }]
    }"#;

    /// Cubism3 模型描述 (Cubism Editor 导出, 名字与路径形式取自真实立绘包)
    const CUBISM3: &str = r#"{
        "Version": 3,
        "Name": "anon",
        "FileReferences": {
            "Moc": "anon.moc3",
            "Textures": ["textures/texture_00.png"],
            "Physics": "anon.physics3.json",
            "Pose": null,
            "DisplayInfo": "anon.cdi3.json",
            "Motions": {
                "mygo/anon/mtn_idle01": [{ "File": "../../../.mtn_exp/mtn/idle01.motion3.json" }]
            },
            "Expressions": [
                { "Name": "mygo/anon/exp_smile01", "File": "../../../.mtn_exp/exp/smile01.exp3.json" }
            ]
        },
        "Groups": [{ "Target": "Parameter", "Name": "EyeBlink", "Ids": ["ParamEyeLOpen", "ParamEyeROpen"] }]
    }"#;

    #[test]
    fn deserializes_cubism2_by_version() {
        let model: Live2dModel = CUBISM2.parse().unwrap();

        assert_eq!(model.get_type(), Live2dModelKind::Cubism2);
        assert_eq!(model.model(), "data/model.moc");
        assert_eq!(model.physics(), Some("data/physics.json"));
        assert_eq!(model.textures(), ["data/textures/texture_00.png"]);
        assert_eq!(model.motions()[0].0, "idle01");
        assert_eq!(model.expressions()[0].name, "idle01");
    }

    #[test]
    fn deserializes_cubism3_by_version() {
        let model: Live2dModel = CUBISM3.parse().unwrap();

        assert_eq!(model.get_type(), Live2dModelKind::Cubism3);
        assert_eq!(model.model(), "anon.moc3");
        assert_eq!(model.physics(), Some("anon.physics3.json"));
        assert_eq!(model.textures(), ["textures/texture_00.png"]);
        assert_eq!(model.motions()[0].0, "mygo/anon/mtn_idle01");
        assert_eq!(
            model.motions()[0].1[0].file,
            "../../../.mtn_exp/mtn/idle01.motion3.json"
        );
        assert_eq!(model.expressions()[0].name, "mygo/anon/exp_smile01");

        // `Pose: null` 与 `Groups` 都要容忍
        let Live2dModel::Cubism3(model) = model else {
            unreachable!()
        };
        assert_eq!(model.assets.pose, None);
        assert_eq!(model.assets.display.as_deref(), Some("anon.cdi3.json"));
        assert_eq!(model.groups[0].name, "EyeBlink");
    }

    /// 方言由 `Version` / `version` 的取值判定, 取值不符一律拒绝
    #[test]
    fn rejects_unknown_version() {
        let cases = [
            "{}".to_string(),
            CUBISM2.replace("\"version\": \"Sample 1.0.0\",", ""),
            CUBISM3.replace("\"Version\": 3,", ""),
            CUBISM2.replace("Sample 1.0.0", "Sample 2.0.0"),
            CUBISM3.replace("\"Version\": 3,", "\"Version\": 2,"),
        ];

        for json in cases {
            let error = serde_json::from_str::<Live2dModel>(&json).unwrap_err();

            assert!(
                error.to_string().contains("无法识别 Live2D 模型描述"),
                "{error}"
            );
        }
    }

    /// 方言判定在序列化后必须仍然成立, 否则 `Display` 与 `FromStr` 的往返会失去类型
    #[test]
    fn roundtrips_both_dialects() {
        for source in [CUBISM2, CUBISM3] {
            let model: Live2dModel = source.parse().unwrap();
            let json = model.to_string();
            let roundtripped: Live2dModel = json.parse().unwrap();

            assert_eq!(model, roundtripped);
        }
    }
}
