//! 单场景语言服务 WASM 封装

use std::ops::Deref;

use lsp_types::*;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use webgal_language_core::{resource::Config, sentence::Scene as SceneInfo};

use crate::{encode::*, project::Project, service::*};

/// WebGAL 场景实例
///
/// 提供语句访问和单场景语言服务.
#[wasm_bindgen]
pub struct Scene(Project);

#[wasm_bindgen]
impl Scene {
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str) -> Self {
        let mut project = Project::new(Config::default());
        project
            .insert("scene/start.txt", || Ok(text.to_string()))
            .unwrap();
        Self(project)
    }

    pub fn sentences(&self) -> Vec<JsValue> {
        self.deref().sentences().iter().map(serialize).collect()
    }

    // -------- service --------

    pub fn diagnose(&self) -> Vec<JsValue> {
        let mut diagnostics = diagnose_project(&self.0)
            .into_iter()
            .find_map(|(path, _, diagnostics)| (path == "start.txt").then_some(diagnostics))
            .unwrap_or_default();
        diagnostics_utf8_to_utf16(&self, &mut diagnostics);

        diagnostics.into_iter().map(serialize).collect()
    }

    pub fn reference(&self, line: u32, character: u32) -> Vec<JsValue> {
        let position = position_utf16_to_utf8(&self, Position { line, character });
        let references = crate::service::reference("start.txt", position, &self.0)
            .unwrap_or_default()
            .into_iter()
            .map(|(path, range)| (path, range_utf8_to_utf16(&self, range)))
            .collect();
        references_to_locations("file:///webgal-playground", references)
            .into_iter()
            .map(serialize)
            .collect()
    }

    /// 悬浮文档
    pub fn document(&self, line: u32, character: u32) -> Option<JsValue> {
        let position = position_utf16_to_utf8(&self, Position { line, character });
        let mut documentation = document(&self, position)?;
        document_utf8_to_utf16(&self, &mut documentation);
        Some(serialize(&documentation))
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
        let mut tokens = highlight(&self);
        highlights_utf8_to_utf16(&self, &mut tokens);

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

    pub fn inlay_hint(&self) -> Vec<JsValue> {
        let mut hints = inlay_hint(
            "start.txt",
            Range {
                end: Position {
                    line: u32::MAX,
                    character: u32::MAX,
                },
                ..Default::default()
            },
            &self.0,
        )
        .unwrap_or_default();
        inlay_hints_utf8_to_utf16(&self, &mut hints);

        hints.into_iter().map(serialize).collect()
    }

    pub fn complete(&self, line: u32, character: u32) -> Vec<JsValue> {
        let position = position_utf16_to_utf8(&self, Position { line, character });

        let mut completions = complete(&self, position, &Project::default());
        completions_utf8_to_utf16(&self, &mut completions);

        completions.into_iter().map(serialize).collect()
    }

    pub fn format(&self) -> Vec<JsValue> {
        let mut edits = format(&self);
        formatting_utf8_to_utf16(&self, &mut edits);

        edits.into_iter().map(serialize).collect()
    }
}

impl Deref for Scene {
    type Target = SceneInfo;

    fn deref(&self) -> &Self::Target {
        self.0
            .resource()
            .scene
            .get("start.txt")
            .unwrap()
            .as_item()
            .unwrap()
    }
}

// -------- util --------

fn serialize<T: Serialize>(value: T) -> JsValue {
    serde_wasm_bindgen::to_value(&value).unwrap()
}
