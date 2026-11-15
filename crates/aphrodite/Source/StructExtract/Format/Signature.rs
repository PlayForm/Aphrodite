//! One rendered signature line, capped.

use crate::StructExtract::{Format::Floor::Fn as Floor, MaximumSignatureLength::Const as MaximumSigLength};

/// Render `Kind` and `Text` as one signature line, truncated to the line cap.
///
/// The two parts are joined with a single space and trimmed, then cut to
/// `MAX_SIG_LEN - 3` bytes with `...` appended when the line overruns, so a
/// signature never exceeds `MaximumSigLength`. It is not a language-aware formatter:
/// the caller passes the already-rendered text (`struct Foo`, `type Bar`) and this
/// only joins and cuts. The cut is byte-safe via `Floor`, and because it does not
/// look for whitespace it can end mid-identifier - the `...` is what tells the
/// model the line was shortened.
pub(crate) fn Fn(Kind:&str, Text:&str) -> String {
	let s = format!("{} {}", Kind, Text).trim().to_string();
	match s.len() > MaximumSigLength {
		true => Floor(&s, MaximumSigLength - 3).to_string() + "...",
		false => s,
	}
}
