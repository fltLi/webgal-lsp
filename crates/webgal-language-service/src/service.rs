use lsp_types::*;

use crate::project::VariableLocation;

pub use complete::*;
pub use diagnose::*;
pub use document::*;
pub use format::*;
pub use highlight::*;
pub use inlay_hint::*;
pub use reference::*;

mod complete;
mod diagnose;
mod document;
mod format;
mod highlight;
mod inlay_hint;
mod reference;

// -------- util --------

fn position_in_range(position: Position, span: Range) -> bool {
    position >= span.start && position < span.end
}

// fn range_in_range(inner: Range, span: Range) -> bool {
//     inner.start >= span.start && inner.end <= span.end
// }

fn variable_location_to_range(location: &VariableLocation) -> Range {
    Range {
        start: Position {
            line: location.line as u32,
            character: location.span.start as u32,
        },
        end: Position {
            line: location.line as u32,
            character: location.span.end as u32,
        },
    }
}
