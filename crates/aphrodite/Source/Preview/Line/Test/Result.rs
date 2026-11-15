//! `test <name> ... ok|FAILED|ignored` result-line predicate.

/// True for a test result line.
///
/// `test ` then a non-empty name then `...` then one of `Okay`, `FAILED` or
/// `Ignored` ending at a word boundary. It is not a result counter - the
/// tallies come from the summary line; this predicate only decides whether a
/// line belongs to the per-test stream.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match t.strip_prefix("test ") {
		Some(r) => r,
		None => return false,
	};
	let NameLength = Rest.chars().take_while(|c| !c.is_whitespace()).count();
	if NameLength == 0 {
		return false;
	}
	let After = Rest[NameLength..].trim_start();
	let After = match After.strip_prefix("...") {
		Some(a) => a,
		None => return false,
	};
	let After = After.trim_start();
	for Kw in ["ok", "FAILED", "ignored"] {
		if let Some(r) = After.strip_prefix(Kw) {
			// word boundary: next char is end or non-word.
			return r.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true);
		}
	}
	false
}
