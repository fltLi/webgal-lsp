use lsp_types::*;

use crate::service::complete::{PrimaryCompletion, make_span};

/// 补全语句注释
pub fn complete_comment(input: &str, position: Position) -> Vec<PrimaryCompletion> {
    if "nolint:".starts_with(input) {
        return vec![PrimaryCompletion {
            name: "nolint".to_string(),
            kind: CompletionItemKind::KEYWORD,
            description: Some("诊断抑制配置".to_string()),
            document: None,
            sort_key: None,
            span: make_span(position, input.len()),
            insert_text: Some("nolint:$1;$0".to_string()),
        }];
    }

    if let Some(input) = input.strip_prefix("nolint:")
        && !input.contains(';')
    {
        let current = match input.rsplit_once('|') {
            Some((_, current)) => current,
            None => input,
        };

        return (1..=16) // 此处需要实时同步最新诊断码编号
            .map(|code| format!("WG{code:03}"))
            .filter(|code| code.starts_with(current))
            .map(|code| PrimaryCompletion {
                name: code,
                kind: CompletionItemKind::ENUM_MEMBER,
                description: None,
                document: None,
                sort_key: None,
                span: make_span(position, current.len()),
                insert_text: None,
            })
            .collect();
    }

    Vec::default()
}
