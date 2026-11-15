//! Blocking-thread wrappers around the CCR backend.
//!
//! The backends are synchronous; calling one from a handler would block the
//! reactor thread. Each verb is its own leaf so the spawn_blocking discipline is
//! visible at the call site rather than buried in a shared helper.
//!
//! Writes are not here: they live in `Storage`, which is the only thing that can
//! produce the evidence a marker is built from. A read, a count and a delete
//! cannot lose content, so they need no evidence type.

pub mod Delete;
pub mod Get;
pub mod Length;
