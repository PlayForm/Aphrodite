//! Blocking-thread wrappers around the CCR backend.
//!
//! The backends are synchronous; calling one from a handler would block the
//! reactor thread. Each verb is its own leaf so the spawn_blocking discipline
//! is visible at the call site rather than buried in a shared helper.

pub mod Delete;
pub mod Get;
pub mod Length;
pub mod Put;
