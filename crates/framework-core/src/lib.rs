//! FrameWork's document model and calculation engine.
//!
//! The crate root is the module tree and a flat re-export of the public
//! surface; every type and every method lives in the module that owns its
//! concern. The declarations below are in dependency order — `model` depends
//! on nothing, `formula` on `model`, `engine` on both, `operation` on all
//! three, and `store` on everything. A `use` that points upward means
//! something landed in the wrong module.

mod error;
mod model;

mod formula;

mod engine;

mod data;
mod operation;
mod validate;

mod collaboration;
mod persist;
mod store;

pub use collaboration::*;
pub use data::*;
pub use engine::*;
pub use error::*;
pub use formula::*;
pub use model::*;
pub use operation::*;
pub use persist::*;
pub use store::*;

// Nothing in `validate` is public API — the checks run inside `apply`, and a
// caller has no way to ask for one on its own.
pub(crate) use validate::*;

pub type Id = String;

#[cfg(test)]
mod test_support;
