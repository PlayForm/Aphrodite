//! Conversational Directives - lightweight behavioral context for the LLM.
//!
//! Directives are short `.md` files that inject behavioral instructions into
//! the LLM's context via `pre_llm_call`. Unlike file content (which gets
//! compressed into CCR markers), directives are **always inline** - they're
//! compact enough to never need compression, and the engine injects them
//! directly into the LLM's context without any file-system round-trip or
//! retrieval step.
//!
//! Per profile, per user: `directives/*.md` files are swappable at runtime
//! via `aphrodite_directive("swap", "name")`.
//!
//! Atomized into a reverse-taxonomy tree: `builtin.rs` (baked-in include_str!
//! defaults), `model.rs` (Directive + size caps), `load.rs` (disk loading),
//! `build.rs` (context injection), `action.rs` (runtime actions),
//! `tests.rs` (battery). This facade re-exports the exact public names the
//! rest of the crate (and `aphrodite-hermes` via `lib.rs`) depends on.

pub mod action;
pub mod build;
pub mod builtin;
pub mod load;
pub mod model;

#[cfg(test)]
mod tests;

pub use action::handle_action;
pub use build::build_directive_context;
pub use builtin::loaded_builtins;
pub use load::load_directives;
pub use model::{Directive, MAX_COMBINED_CHARS, MAX_DIRECTIVE_CHARS};
