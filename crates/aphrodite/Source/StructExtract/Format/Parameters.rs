//! One parameter list, capped.

use std::num::NonZeroUsize;

use crate::StructExtract::{Format::Floor::Fn as Floor, MaximumParametersLength::Const as MaximumParametersLength};

/// Truncate a parameter list to `MaximumParametersLength` bytes with `...` appended.
///
/// The extractors pass the text between the parentheses verbatim; this cuts it to `Maximum - 3` bytes
/// plus three dots, so the list never exceeds the cap. It is not a parser: it does not balance
/// parentheses, split parameters or check syntax, so `a: (i32, i32)` is one opaque string and a cut can
/// land inside a nested tuple. That is the accepted cost of keeping the preview cheap; what must not
/// drift is the cap, because the parameter list is most of a signature's width and the line cap is
/// measured after it.
///
/// A list whose length EQUALS the cap is returned whole. `checked_sub` read equality as an overrun, so
/// a list at exactly 35 bytes lost its last three characters to the ellipsis - the same size, three
/// characters less information, which is the defect class this preview exists to avoid.
pub(crate) fn Fn(Params:&str) -> String {
	match NonZeroUsize::new(Params.len().saturating_sub(MaximumParametersLength)) {
		None => Params.to_string(),
		Some(_) => Floor(Params, MaximumParametersLength - 3).to_string() + "...",
	}
}
