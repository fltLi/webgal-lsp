//! 单场景语言服务 WASM 封装

use lsp_types::*;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_language_core::sentence::Scene as SceneInfo;

use crate::{encode::*, project::Project, service::*};

/// WebGAL 场景实例
///
/// 提供语句访问和单场景语言服务.
#[wasm_bindgen]
pub struct Scene {
    scene: SceneInfo,
}

#[wasm_bindgen]
impl Scene {
    #[wasm_bindgen(constructor)]
    pub fn parse(text: &str) -> Self {
        let scene = SceneInfo::from_str(text);
        Self { scene }
    }

    pub fn sentences(&self) -> Vec<JsValue> {
        self.scene.sentences().iter().map(serialize).collect()
    }

    // -------- service --------

    pub fn diagnose(&self) -> Vec<JsValue> {
        let mut diagnostics = diagnose_scene(&self.scene, None);
        diagnostics_utf8_to_utf16(&self.scene, &mut diagnostics);

        diagnostics.into_iter().map(serialize).collect()
    }

    pub fn highlight_token_types() -> Vec<String> {
        token_types()
            .iter()
            .map(|token_type| token_type.as_str().to_string())
            .collect()
    }

    /// 提供场景高亮
    ///
    /// # Returns
    /// [`SemanticToken`] 数组, 每五个整型分别表示:
    /// * `delta_line`.
    /// * `delta_start`.
    /// * `length`.
    /// * `token_type`.
    /// * `token_modifiers_bitset`.
    pub fn highlight(&self) -> Vec<u32> {
        let mut tokens = highlight(&self.scene);
        highlights_utf8_to_utf16(&self.scene, &mut tokens);

        tokens
            .into_iter()
            .flat_map(|token| {
                [
                    token.delta_line,
                    token.delta_start,
                    token.length,
                    token.token_type,
                    token.token_modifiers_bitset,
                ]
            })
            .collect()
    }

    pub fn complete(&self, line: u32, character: u32) -> Vec<JsValue> {
        let position = position_utf16_to_utf8(&self.scene, Position { line, character });

        let mut completions = complete(&self.scene, position, &Project::default());
        completions_utf8_to_utf16(&self.scene, &mut completions);

        completions.into_iter().map(serialize).collect()
    }

    pub fn format(&self) -> Vec<JsValue> {
        let mut edits = format(&self.scene);
        formatting_utf8_to_utf16(&self.scene, &mut edits);

        edits.into_iter().map(serialize).collect()
    }
}

// -------- util --------

fn serialize<T: Serialize>(value: T) -> JsValue {
    serde_wasm_bindgen::to_value(&value).unwrap()
}
