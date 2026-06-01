//! Views, geometry, and the downward-context substrate.
//!
//! Carries the geometry primitives ([`Point`]/[`Rect`], rows 1–2), the
//! [`ViewId`] generational arena ([`id`], D3 row 17), and the downward
//! [`Context`] / [`DrawCtx`] types ([`context`], D3/D4 row 22). The `View` trait
//! + `ViewState` (`TView`, D2/D5) land here in Phase 1 (row 23).

mod context;
mod geometry;
mod id;

pub use context::{Context, DrawCtx};
pub use geometry::{Point, Rect};
pub use id::{ViewArena, ViewId};
