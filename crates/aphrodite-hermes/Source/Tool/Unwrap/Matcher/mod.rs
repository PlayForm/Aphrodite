//! One leaf per envelope shape, each a named function of the same signature.
//!
//! A matcher answers one question - "is this object that shape, and if so what
//! is its content and type?" - and returns `None` when it is not, so `Fn/`
//! composes them with `or_else` and position is the only priority that exists.
//! Extraction inside a matcher uses `?` and `filter`: a guarded `match` would
//! put the same predicate in two places (the guard and the fall-through arm)
//! and would evaluate its alternatives eagerly.

pub mod Error;
pub mod Patch;
pub mod Read;
pub mod Search;
pub mod Single;
pub mod Success;
pub mod Terminal;
