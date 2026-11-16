//! The build stamp: which image is answering, not just which version.
//!
//! The values come from `crates/aphrodite-hermes/build.rs`, which stamps them as
//! `cargo:rustc-env` on every build - the core crate's `APHRODITE_*` stamp does
//! not reach this crate, because `rustc-env` applies only to the crate whose
//! build script emitted it.

pub mod Fn;
pub mod Identity;
pub mod Struct;
