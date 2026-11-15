//! Rhai scripting engine - user-defined micro-scripts.
//!
//! Port of the pre-Rust `scripting.rs`, feature-gated behind the `scripting`
//! Cargo feature (`pub mod Scripting` in `Library.rs` is only compiled with the
//! feature on, so the default cdylib build never links rhai).
//!
//! Contract: a script is a `.rhai` file declaring a zero-argument hook
//! function (e.g. `fn on_compress() { ... }`). The engine compiles scripts
//! per call (no AST storage - avoids Send issues), seeds a scope with the
//! hook payload, calls the named function, and chains each script's output
//! into the next script's `Content`. Any script error is warned and skipped
//! (degrade to the previous value) - a failing script must never crash the
//! proxy.
//!
//! The tree is three nouns and their behaviour: `Payload/` is what a script
//! sees, `Script/` is one loaded file, and `Engine/` is the loader and caller -
//! its 159-line impl split by concern (construction, reload, the mtime check,
//! and one file per hook surface).
//!
//! The rhai `Engine` type is imported as `Rhai` throughout: the module that
//! owns it is now itself named `Engine`, and a bare `Engine::new()` inside
//! `Scripting::Engine` would read as the module. The alias is the only name
//! change in the port.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Scripting::Engine::Struct::Struct`), so the file that defines an item is
//! the only place it is defined and there is exactly one way to spell it.

pub mod Engine;
pub mod Payload;
pub mod Script;

#[cfg(test)]
mod Test;
