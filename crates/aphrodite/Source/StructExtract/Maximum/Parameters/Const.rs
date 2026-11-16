//! The cap on one parameter list.
//!
//! Named `Const` because the path carries the domain, per the house style.
#![allow(non_upper_case_globals)]

/// Characters a parameter list may occupy before truncation: 35.
///
/// The Rust, Python and Go extractors render `name(params)` and truncate the
/// parenthesised text to this before measuring the whole signature against
/// `Maximum::Signature::Const`. It is not a parse of the parameters and it
/// does not balance parentheses; it cuts at a character boundary so the list ends
/// in `...` instead of a broken codepoint. Made much smaller, every signature
/// collapses to `fn name(...)` and the model loses the arity the preview exists to
/// carry.
pub(crate) const Const:usize = 35;
