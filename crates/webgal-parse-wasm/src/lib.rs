#[cfg(feature = "highlight")]
use lsp_types::SemanticToken;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_model::sentence::SentenceInfo;

/// 解析场景, 以 [`JsValue`] 格式返回数据结构
#[wasm_bindgen]
pub fn parse_scene(text: &str) -> Result<Vec<JsValue>, JsValue> {
    text.lines()
        .map(|line| serialize(&SentenceInfo::from_str(line)))
        .collect()
}

/// 解析语句, 以 [`JsValue`] 格式返回数据结构
#[wasm_bindgen]
pub fn parse_sentence(text: &str) -> Result<JsValue, JsValue> {
    let sentence = SentenceInfo::from_str(text);
    serialize(&sentence)
}

/// 提供场景语义高亮, 返回 [`Vec<lsp_types::SemanticToken>`]
#[cfg(feature = "highlight")]
#[wasm_bindgen]
pub fn highlight_scene(text: &str) -> Result<JsValue, JsValue> {
    use webgal_highlight::highlight;
    use webgal_model::sentence::Scene;

    let scene = Scene::from_str(text);
    let tokens = highlight(&scene);

    serialize(
        &tokens
            .into_iter()
            .map(SemanticTokenHelper::from)
            .collect::<Vec<_>>(),
    )
}

// -------- util --------

fn serialize<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(From::from)
}

#[cfg(feature = "highlight")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticTokenHelper {
    delta_line: u32,
    delta_start: u32,
    length: u32,
    token_type: u32,
    token_modifiers_bitset: u32,
}

#[cfg(feature = "highlight")]
impl From<SemanticToken> for SemanticTokenHelper {
    fn from(value: SemanticToken) -> Self {
        let SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset,
        } = value;
        Self {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset,
        }
    }
}
