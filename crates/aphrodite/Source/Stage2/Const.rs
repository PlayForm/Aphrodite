//! The Stage-2 size floor.
//!
//! Reduction has a fixed cost (a parse, a scan, a rebuilt string) and the
//! marker that replaces the content costs bytes of its own, so content below
//! this size cannot come out smaller - it would only lose information. Keeping
//! the floor in its own file makes the one number that decides "is this worth
//! reducing at all" impossible to miss or to duplicate.

/// Smallest content size that is reduced at all: eighty bytes. Below it the marker
/// and header overhead would make the result larger than the input.
pub(crate) const Minimum:usize = 80;
