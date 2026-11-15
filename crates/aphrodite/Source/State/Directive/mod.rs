//! Ephemeral directive activations - the one-shot and TTL nudges.
//!
//! A named directive lives in `state.directives` and is activated by name; an
//! ephemeral entry is different in kind: it carries literal text synthesized
//! by a feature at the moment it is needed (error-loop breaker, redundant-read
//! deflector, auto-swap announce) and expires on its own. Keeping the type
//! here rather than in `crate::Directive` is what stops the ephemeral path from
//! growing a name-resolution path it does not have.

pub mod Struct;

pub use Struct::ActiveDirective;
