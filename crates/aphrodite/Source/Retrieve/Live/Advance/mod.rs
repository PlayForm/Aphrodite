//! Advance a live container's progress.
//!
//! Two halves, because they fail differently: `Fn/` computes the progress value
//! and cannot fail, `Write/` persists it and must not be able to fail the request.

pub mod Fn;
pub mod Write;
