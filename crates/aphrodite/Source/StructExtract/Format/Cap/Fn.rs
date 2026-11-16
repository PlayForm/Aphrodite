//! Cap one already-rendered signature line.

use std::num::NonZeroUsize;

use crate::StructExtract::{Format::Floor::Fn as Floor, Maximum::Signature::Const::Const as Maximum};

/// Cut one rendered line to `Maximum::Signature::Const` bytes, appending `...` when it overruns.
///
/// This is the ONLY place a rendered signature is measured against the line cap. It used to exist
/// twice - once here and once inline in `Render` - and the two disagreed at the boundary: one treated
/// a line exactly at the cap as an overrun and replaced its last three characters with dots. One
/// measurement, one boundary.
///
/// It is not a word-aware cut: the line is cut where the cap falls, and the `...` is what tells the
/// model the line was shortened.
pub(crate) fn Fn(Rendered:String) -> String {
	match NonZeroUsize::new(Rendered.len().saturating_sub(Maximum)) {
		None => Rendered,
		Some(_) => Floor(&Rendered, Maximum - 3).to_string() + "...",
	}
}
