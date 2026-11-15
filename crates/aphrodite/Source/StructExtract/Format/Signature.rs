//! One rendered signature line, capped.

use crate::StructExtract::{Format::Floor::Fn as Floor, MaximumSignatureLength::Const as MaximumSignatureLength};

/// Render `Kind` and `Text` as one signature line, truncated to the line cap.
///
/// The two parts are joined with a single space and trimmed, then cut to `Maximum - 3` bytes with `...`
/// appended when the line overruns, so a signature never exceeds `MaximumSignatureLength`. It is not a
/// language-aware formatter: the caller passes the already-rendered text (`struct Foo`, `type Bar`) and
/// this only joins and cuts. The cut is byte-safe via `Floor`, and because it does not look for
/// whitespace it can end mid-identifier - the `...` is what tells the model the line was shortened.
pub(crate) fn Fn(Kind:&str, Text:&str) -> String {
	let Rendered = format!("{Kind} {Text}").trim().to_string();

	match Rendered.len().checked_sub(MaximumSignatureLength) {
		None => Rendered,
		Some(_) => Floor(&Rendered, MaximumSignatureLength - 3).to_string() + "...",
	}
}
