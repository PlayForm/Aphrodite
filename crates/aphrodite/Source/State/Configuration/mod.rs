//! The resolved configuration the session mirrors.
//!
//! - `Struct.rs` - every value mirrored out of `aphrodite.toml`.
//! - `Default.rs` - the seed a session has before `Apply::Startup::Fn` runs.

pub mod Default;
pub mod Struct;
