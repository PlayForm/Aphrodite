//! The config-seeded state template, its reload path, and the call-path
//! prediction the dev-only hook log reports.
//!
//! This crate never reads `aphrodite.toml` per call: `Template/` builds the
//! config-seeded `AphroditeState` once per process and every session is a
//! clone of it, `Reload/` re-applies a freshly loaded config to a live
//! session while preserving the runtime-owned fields, and `Path/` predicts
//! which compression path a tool call will take from those config fields.

pub mod Path;
pub mod Reload;
pub mod Template;

#[cfg(test)]
mod Test;
