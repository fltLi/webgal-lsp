use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_model::sentence::SentenceInfo;

/// 解析场景, 以 JSON 格式返回数据结构
#[wasm_bindgen]
pub fn parse_scene(text: &str) -> String {
    let scene: Vec<_> = text.lines().map(SentenceInfo::from_str).collect();
    serialize(&scene)
}

/// 解析语句, 以 JSON 格式返回数据结构
#[wasm_bindgen]
pub fn parse_sentence(text: &str) -> String {
    let sentence = SentenceInfo::from_str(text);
    serialize(&sentence)
}

// -------- util --------

fn serialize<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or(r#"{"error":"serialization failed"}"#.to_string())
}
