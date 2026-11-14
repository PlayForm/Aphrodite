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

pub mod Action;
pub mod Build;
pub mod Builtin;
pub mod Load;
pub mod Model;

#[cfg(test)]
mod Test;

pub use Action::handle_action;
pub use Build::build_directive_context;
pub use Builtin::loaded_builtins;
pub use Load::load_directives;
pub use Model::{Directive, MAX_COMBINED_CHARS, MAX_DIRECTIVE_CHARS};
