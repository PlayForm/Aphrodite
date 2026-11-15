//! The nested-marker walk, and its size-bounded variant.
//!
//! Resolution is recursive because a stored entry may itself contain markers, and
//! those entries may contain more. Two things make that safe: the `Visited` set,
//! which stops a cycle instead of overflowing the stack, and the configured depth
//! limit, which stops a chain from expanding forever. Both are parameters rather
//! than module state, so one retrieval's guard can never leak into the next.
//!
//! - `Recursive.rs` - the walk itself: resolve, scan, substitute, recurse.
//! - `Inline.rs` - the auto-expand question: is the resolved content small
//!   enough to inline in place of the marker?
//!
//! The split exists because the two have different contracts: the walk always
//! returns what it could resolve, while the auto-expand variant must return
//! nothing at all when the result is too large, so the caller keeps the marker.
//!
//! `clippy::module_inception` is allowed here on purpose: the walk IS the
//! recursive resolver, and `Recursive/Recursive.rs` names the domain the same way
//! `Configuration/Proxy/Proxy/` does. The alternative is a synonym that says the
//! same thing less precisely.
#![allow(clippy::module_inception)]

pub mod Inline;
pub mod Recursive;
