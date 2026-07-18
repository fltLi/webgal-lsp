//! 单脚本语言服务 WASM 封装

use std::result;

use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_language_core::sentence::Scene as SceneInfo;

use crate::{
    encode::highlights_utf8_to_utf16,
    service::{highlight, token_types},
};

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

    pub fn sentences(&self) -> Result<Vec<JsValue>> {
        self.scene.sentences().iter().map(serialize).collect()
    }

    // -------- service --------

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
}

// -------- util --------

type Result<T> = result::Result<T, JsValue>;

fn serialize<T: Serialize>(value: &T) -> Result<JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(From::from)
}
