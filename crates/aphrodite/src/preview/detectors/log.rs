//! log shape detector (Phase 6: rules lifted verbatim from the old proxy
//! classifier `proxy.rs:1462-1478`, re-expressed line-based).

use crate::preview::input::Input;

/// Log output: explicit `[LEVEL]` markers (`[INFO]`, `[WARN]`, `[ERROR]`,
/// `[DEBUG]`, `[TRACE]`, `[FATAL]`, `[PANIC]`) or a timestamp-prefixed line
/// (starts with a digit, >10 chars, contains `:` or `-`). Same rules the
/// proxy classifier used; lives in the pipeline now so both paths agree.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with('[')
			&& (t.contains("INFO")
				|| t.contains("WARN")
				|| t.contains("ERROR")
				|| t.contains("DEBUG")
				|| t.contains("TRACE")
				|| t.contains("FATAL")
				|| t.contains("PANIC"))
	}) || inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with(|c:char| c.is_ascii_digit()) && t.len() > 10 && (t.contains(':') || t.contains('-'))
	})
}