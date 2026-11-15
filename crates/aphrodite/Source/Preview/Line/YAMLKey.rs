//! Top-level YAML key-line predicate.
//!
//! YAML is recognised by unindented `key: value` lines. The trap is a compiler
//! log, which also prints `error: ...` at column zero - so the predicate has to
//! exclude the log-marker keys or every rustc run would preview as YAML.

/// True for a top-level YAML key line (`name: webapp`).
///
/// A lowercase identifier key at column zero, with or without a value. The
/// log-marker keys `error`, `warning`, `Note`, `info` and `warn` are excluded
/// so a compiler log is never mis-tagged as YAML; `debug:` and `trace:` are
/// legitimate YAML keys and stay accepted. It is not a YAML parser and nested
/// keys are deliberately ignored - only top-level keys vote.
pub(crate) fn Fn(Line:&str) -> bool {
	if Line.starts_with(' ') || Line.as_bytes().first() == Some(&9) {
		return false;
	}
	let t = Line.trim_end();
	let Index = match t.find(':') {
		Some(i) => i,
		None => return false,
	};
	if Index == 0 || Index > 64 {
		return false;
	}
	let Key = &t[..Index];
	if !Key
		.chars()
		.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
	{
		return false;
	}
	// Exclude log-marker keys (`error:`/`warning:`/...) so a compiler log is
	// never mis-tagged as yaml. `debug:`/`trace:` are legitimate yaml keys.
	!matches!(Key, "error" | "warning" | "note" | "info" | "warn")
}
