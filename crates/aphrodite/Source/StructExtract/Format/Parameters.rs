//! One parameter list, capped.

use crate::StructExtract::{Format::Floor::Fn as Floor, MaximumParametersLength::Const as MaximumParamsLength};

/// Truncate a parameter list to `MaximumParamsLength` characters with `...` appended.
///
/// The extractors pass the text between the parentheses verbatim; this cuts it to
/// `MAX_PARAMS_LEN - 3` bytes plus three dots, so the list never exceeds the cap.
/// It is not a parser: it does not balance parentheses, split parameters or check
/// syntax, so `a: (i32, i32)` is one opaque string and a cut can land inside a
/// nested tuple. That is the accepted cost of keeping the preview cheap; what must
/// not drift is the cap, because the parameter list is most of a signature's
/// width and the line cap is measured after it.
pub(crate) fn Fn(Params:&str) -> String {
	if Params.len() > MaximumParamsLength {
		Floor(Params, MaximumParamsLength - 3).to_string() + "..."
	} else {
		Params.to_string()
	}
}
