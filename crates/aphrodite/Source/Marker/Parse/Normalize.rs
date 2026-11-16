//! Hash normalization at the retrieve boundary.
//!
//! The model is not required to echo a bare hash: it may hand back the whole
//! marker body (`hash|type|size`) or pad it with whitespace it picked up from
//! the rendered line. Normalizing once, at the entry point, means every
//! retrieval site tolerates the same inputs instead of each one re-inventing
//! a trim.

/// Normalize a hash argument handed to a retrieve-style entry point.
///
/// External contract: `Hash` is whatever the caller passed as a hash - possibly
/// a whole marker body or a whitespace-padded value - and the answer is the bare
/// hash, borrowed from the input. An empty or all-whitespace argument normalizes
/// to `""`, which the callers then reject as unknown; there is no `None` case.
///
/// LLMs sometimes echo back a whole marker body (`hash|type|size`) instead of
/// the bare hash, or wrap it in incidental whitespace (report 05 F3). Strip
/// everything from the first `|` onward and trim surrounding whitespace so
/// every retrieval site tolerates the same inputs `Resolve::One::Fn` already
/// does. Idempotent: normalizing an already-bare hash is a no-op.
pub fn Fn(Hash:&str) -> &str { Hash.split('|').next().unwrap_or(Hash).trim() }
