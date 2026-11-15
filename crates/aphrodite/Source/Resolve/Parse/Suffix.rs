//! The marker closing delimiter.
//!
//! The counterpart of the opening delimiter: a marker is only a marker when it is
//! closed, which is what lets content that merely mentions the opening text stay
//! inert. It is not a bracket matcher - the grammar does not nest markers inside
//! markers, so the first `>>>` after the prefix ends the marker.
//!
//! Named `Const` because the file name carries the domain, per the house style.
#![allow(non_upper_case_globals)]

/// The text every CCR marker ends with.
///
/// The scanner looks for the first occurrence after the prefix; text after it is
/// content again. It is not optional and there is no unterminated-marker form: an
/// unclosed prefix is skipped rather than guessed at, because guessing would
/// resolve a marker the writer never finished emitting.
pub(crate) const Const:&str = ">>>";
