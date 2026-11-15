//! One entry in the first-line rule table.
//!
//! A named alias rather than the tuple type spelled at the const: the table's
//! order is its contract, and a reader should see "a rule" once instead of
//! decoding `(&str, fn(&str) -> bool)` at every mention.

/// A rule: the content type it claims, and the predicate that recognises it.
pub(crate) type Type = (&'static str, fn(&str) -> bool);
