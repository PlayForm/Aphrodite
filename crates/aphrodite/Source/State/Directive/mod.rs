//! Ephemeral directive activations - the one-shot and TTL nudges.
//!
//! A named directive lives in `state.directives` and is activated by name; an
//! ephemeral entry is different in kind: it carries literal text synthesized
//! by a feature at the moment it is needed (error-loop breaker, redundant-read
//! deflector, auto-swap announce) and expires on its own. Keeping the type
//! here rather than in `crate::Directive` is what stops the ephemeral path from
//! growing a name-resolution path it does not have.
//!
//! - `Struct.rs` - the loaded set, the active names, the nudges and the latch.
//! - `Default.rs` - the seed: nothing loaded, nothing active, no latch.
//! - `Entry/` - one ephemeral activation (the item).

pub mod Default;
pub mod Entry;
pub mod Struct;
