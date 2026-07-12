use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_model::sentence::SentenceInfo;

/// 解析场景, 以 [`JsValue`] 格式返回数据结构
#[wasm_bindgen]
pub fn parse_scene(text: &str) -> Result<JsValue, JsValue> {
    let scene: Vec<_> = text.lines().map(SentenceInfo::from_str).collect();
    serialize(&scene)
}

/// 解析语句, 以 [`JsValue`] 格式返回数据结构
#[wasm_bindgen]
pub fn parse_sentence(text: &str) -> Result<JsValue, JsValue> {
    let sentence = SentenceInfo::from_str(text);
    serialize(&sentence)
}

// -------- util --------

fn serialize<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(From::from)
}
