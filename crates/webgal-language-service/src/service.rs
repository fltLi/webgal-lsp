use lsp_types::*;

pub use complete::*;
pub use diagnose::*;
pub use document::*;
pub use format::*;
pub use highlight::*;
pub use inlay_hint::*;

mod complete;
mod diagnose;
mod document;
mod format;
mod highlight;
mod inlay_hint;

// -------- util --------

fn position_in_range(position: Position, span: Range) -> bool {
    position >= span.start && position < span.end
}

// fn range_in_range(inner: Range, span: Range) -> bool {
//     inner.start >= span.start && inner.end <= span.end
// }
