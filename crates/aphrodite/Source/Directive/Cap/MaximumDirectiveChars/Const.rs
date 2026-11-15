//! Per-file cap on directive content.

/// Per-file cap applied when a directive `.md` is loaded from disk.
///
/// Expressed and measured in CHARACTERS. The comparison used to be against
/// `Content.len()` - a byte length - so a directive that fit the character budget but not
/// a byte budget was cut anyway; the combined cap already compares characters for
/// exactly this reason (T19). Both loading paths and the built-in set now go through
/// [`super::super::Fn::Fn`], so there is one place this measure can be wrong.
pub(crate) const Const:usize = 2000;
