//! Compass — durable planning intent for coding agents.
//!
//! Compass owns two layers with different regimes: immutable, content-addressed
//! Plan Versions carrying structural intent, and append-only Progress Events
//! carrying execution. Head, Readiness and lineage are derived, never stored.
//!
//! Zero external crates, deliberately: it keeps the build offline-capable and
//! avoids baking in a serialization format while DQ02 is open.

pub mod block;
pub mod catalog;
pub mod chain;
pub mod change;
pub mod cli;
pub mod cmd;
pub mod convergence;
pub mod event;
pub mod json;
pub mod model;
pub mod predicate;
pub mod readiness;
pub mod refs;
pub mod sha256;
pub mod style;
pub mod version;
