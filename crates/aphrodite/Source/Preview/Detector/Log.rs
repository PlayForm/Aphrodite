//! Log shape detector.
//!
//! Logs are recognised by level markers or by timestamped lines. The rules here
//! are the ones the old proxy classifier used, re-expressed line-based so both
//! paths agree. It is not a log parser: levels are matched, never interpreted.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like log output.
///
/// An explicit `[LEVEL]` marker (`[INFO]`, `[WARN]`, `[ERROR]`, `[DEBUG]`,
/// `[TRACE]`, `[FATAL]`, `[PANIC]`) or a timestamp-prefixed line (starts with a
/// digit, longer than ten characters, contains `:` or `-`). It is not a
/// structured-log reader: nothing is parsed into fields.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	Inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with('[')
			&& (t.contains("INFO")
				|| t.contains("WARN")
				|| t.contains("ERROR")
				|| t.contains("DEBUG")
				|| t.contains("TRACE")
				|| t.contains("FATAL")
				|| t.contains("PANIC"))
	}) || Inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with(|c:char| c.is_ascii_digit()) && t.len() > 10 && (t.contains(':') || t.contains('-'))
	})
}
