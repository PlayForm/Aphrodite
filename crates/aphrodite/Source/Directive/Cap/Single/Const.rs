//! Per-file cap on directive content.

/// Per-file cap applied when a directive `.md` is loaded from disk.
///
/// `Cap::Single` is the per-directive number; `Cap::Combined` is the cap on the text
/// several of them stack up to. `0` is not a sentinel - a cap of zero would cut every
/// body to an ellipsis, and the value is a constant rather than configuration.
///
/// Expressed and measured in CHARACTERS. The comparison used to be against
/// `Content.len()` - a byte length - so a directive that fit the character budget but not
/// a byte budget was cut anyway; the combined cap already compares characters for
/// exactly this reason (T19). Both loading paths and the built-in set now go through
/// [`super::super::Fn::Fn`], so there is one place this measure can be wrong.
pub(crate) const Const:usize = 2000;
