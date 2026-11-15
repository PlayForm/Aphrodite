//! Stage 2 compression - semantic reduction of CCR-stored content.
//!
//! Produces a denser version of a piece of content on demand via the stateless
//! `aphrodite_stage2(content, ccr_type)` ABI (see `Library.rs::aphrodite_stage2`) - it takes
//! content directly and returns the reduced string, with no interaction with the CCR store or
//! the session's inline store.
//!
//! There is no depth-aware retrieval: no `AphroditeRetrieve` / `ResolveOne` /
//! `Resolve::Expand::Fn` entry point accepts a `Depth` parameter, and nothing in this crate
//! writes a stage-2-reduced value back into the store under a `{hash}#stage2` key for a later
//! plain retrieval to pick up. Wiring depth-aware retrieval up for real is a deliberate feature
//! decision, not something this module does today.
//!
//! The reduction is an OUTCOME, not an `Option`: "too small to bother", "no reducer registered
//! for this type", "the reducer did not apply" and "it applied but saved nothing" are four
//! different answers and only one of them is the caller's business. `Outcome` names all five
//! (including the accepted reduction), and `Reduction` is a type only a strictly-shorter
//! candidate can become - so "a reduction exists implies it is shorter" is a fact about the
//! type rather than a check every caller has to remember.
//!
//! Tree:
//!
//! - `Const.rs` - the size floor below which no reduction is attempted.
//! - `Reducer/Type.rs` - the `fn(&str) -> Option<String>` shape of a reducer.
//! - `Reduce/<Family>/` - one directory per content family, each with its own recognisers.
//! - `Reduce/Sample/Struct.rs` - the bounded sample that keeps an exact total.
//! - `Outcome/Enum.rs` - the five answers the entry point can give.
//! - `Reduction/Struct.rs` - an accepted, strictly shorter replacement.
//! - `Registry/Fn.rs` - CCR type string to reducer.
//! - `Compress/Decide/Fn.rs` - the gates, as a value.
//! - `Compress/Fn.rs` - the public `Option<String>` adapter.
//!
//! This facade declares the tree. Nothing else belongs in this file.

pub mod Compress;
pub mod Const;
pub mod Name;
pub mod Outcome;
pub mod Reduce;
pub mod Reducer;
pub mod Reduction;
pub mod Registry;

#[cfg(test)]
pub mod Test;
