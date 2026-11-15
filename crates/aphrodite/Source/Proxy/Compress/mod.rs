//! Rewriting a Chat Completions response so oversized tool output becomes a
//! retrievable marker.
//!
//! The pass is a decision per message (`Plan`) and an application of those
//! decisions (`Apply`), and its result is a `Rewrite` per message - never a
//! boolean beside a mutated body. That distinction is load-bearing: content can
//! be stored without reducing the wire response (auto-expand, inline storage),
//! and a `bool` could not say which had happened.

pub mod Chat;
