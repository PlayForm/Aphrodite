//! The marker registration: one value, one publication.
//!
//! What a marker render needs from the configuration is a grammar and a hint,
//! and it needs both at once. Keeping them as three process-globals (a format
//! `Option<String>`, a hint `String` and an `AtomicBool` knob) allowed a render
//! to observe a format from one registration and a hint from the next; keeping
//! them as one value under one lock makes that mixture unrepresentable.
//!
//! - `Struct/` - the published value: `Grammar` plus `Hint`.
//! - `Const/` - the one static.
//! - `Set/` - the writer, and the only place the configuration's three-key
//!   surface is interpreted.
//! - `With/` - the reader, a closure so nothing is cloned per render.
//! - `Guard/` - the test serialisation, compiled only under `cfg(test)`.

pub mod Const;
#[cfg(test)]
pub mod Guard;
pub mod Set;
pub mod Struct;
pub mod With;
