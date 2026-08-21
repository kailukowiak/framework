pub mod build;
pub mod cache;
pub mod compute;
pub mod plan;
mod recurrence;
mod scratchwork;
mod style;
mod summary;
pub mod trace;
pub mod values;

// `frame` holds `impl FrameObject`, not types, and its name would collide with
// `model::frame` in the crate root's glob re-exports.
pub(crate) mod frame;

// `cache` is derived state, internal to the crate; `compute` carries the public
// projections (DocumentView, FramePage, FrameQueryPlan) that the desktop and
// MCP consumers name directly.
pub(crate) use cache::*;
pub use compute::*;
pub use plan::Layer;
pub(crate) use plan::in_plain_words;
pub(crate) use style::StyleRuleMatches;
pub use summary::*;
pub use trace::*;
pub(crate) use values::*;
