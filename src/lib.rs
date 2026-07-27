//! Compass — durable planning intent for coding agents.
//!
//! Compass owns two layers with different regimes: immutable, content-addressed
//! Plan Versions carrying structural intent, and append-only Progress Events
//! carrying execution. Head, Readiness and lineage are derived, never stored.
//!
//! A version is a TypeScript module (decision 0014); reading it is evaluating
//! it in an embedded, capability-free JavaScript engine (see [`eval`]).
//! rquickjs (QuickJS-ng) and oxc are the crate's only external dependencies,
//! authorised by decision 0011.

pub mod block;
pub mod catalog;
pub mod chain;
pub mod cli;
pub mod cmd;
pub mod convergence;
pub mod eval;
pub mod event;
pub mod json;
pub mod model;
pub mod predicate;
pub mod readiness;
pub mod refs;
pub mod sha256;
pub mod style;
pub mod version;
