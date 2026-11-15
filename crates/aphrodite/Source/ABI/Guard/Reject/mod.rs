//! The lazy rejection primitive.
//!
//! A predicate that has to produce an error is a `bool` that builds its error
//! only when it rejects. Writing it as `if Cond { bail!() }` at every call site
//! puts the boolean back in the consumer and lets a later editor drop one rule
//! of a sequence; writing it here keeps the boolean in one leaf and makes a
//! rejection sequence read as ordered `?` statements.
//!
//! It lives under `ABI::Guard` because the ABI is its only consumer, and because
//! `crate::Guard` is the BINARY's management-route auth boundary (`Main.rs`
//! declares it; `Library.rs` does not), so the library cannot reach it. The
//! style document's "move it to a top-level `Reject/` leaf once a second
//! non-ABI consumer appears" still stands - that move is a `Library.rs`
//! declaration, which is outside this module's ownership.

pub mod Fn;
