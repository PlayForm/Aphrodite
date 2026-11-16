//! The nested-marker walk, and its size-bounded variant.
//!
//! Resolution is recursive because a stored entry may itself contain markers, and those
//! entries may contain more. Two things make that safe: the traversal's PATH, which stops a
//! cycle instead of overflowing the stack, and the configured depth limit, which stops a
//! chain from expanding forever. Both belong to the traversal that owns them, so one
//! retrieval's guard cannot leak into the next - and the path is left as well as entered, so
//! "already on the path" means a cycle rather than "seen anywhere in this tree".
//!
//! - `Prior/` - what the walk already knows about a hash: cycle, depth cap, cache.
//! - `Walk/` - the fresh case: enter the path, resolve, scan, substitute, leave it.
//! - `Recursive.rs` - the dispatch that turns `Prior/` into one of the three answers or hands
//!   the hash to `Walk/`.
//! - `Inline/` - the auto-expand question: is the resolved content small enough to inline in
//!   place of the marker?
//!
//! The last split exists because the two entry points have different contracts: the walk
//! always returns what it could resolve, while the auto-expand variant must return nothing at
//! all when the result is too large, so the caller keeps the marker. That is also why the
//! budget is a property of the traversal rather than a check at the end: the bounded entry
//! point hands the cap down and every level of the walk honours it while it builds.
//!
//! `clippy::module_inception` is allowed here on purpose: the walk IS the recursive resolver,
//! and `Recursive/Recursive.rs` names the domain the same way `Configuration/Proxy/Proxy/`
//! does. The alternative is a synonym that says the same thing less precisely.
#![allow(clippy::module_inception)]

pub mod Inline;
pub mod Prior;
pub mod Recursive;
pub mod Walk;
