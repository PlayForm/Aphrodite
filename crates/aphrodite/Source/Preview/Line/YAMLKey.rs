//! Top-level YAML key-line predicate.
//!
//! YAML is recognised by unindented `key: value` lines. The trap is a compiler log, which also
//! prints `error: ...` at column zero - so the predicate has to exclude the log-marker keys or
//! every rustc run would preview as YAML.

/// Longest key a top-level YAML key line may carry: sixty-four characters.
const KeyMaximum:usize = 64;

/// Keys a compiler log prints at column zero, which are not YAML keys.
const LogMarkers:[&str; 5] = ["error", "warning", "note", "info", "warn"];

/// True for a top-level YAML key line (`name: webapp`).
///
/// A lowercase identifier key at column zero, with or without a value. The log-marker keys
/// `error`, `warning`, `note`, `info` and `warn` are excluded so a compiler log is never
/// mis-tagged as YAML; `debug:` and `trace:` are legitimate YAML keys and stay accepted. It is
/// not a YAML parser and nested keys are deliberately ignored - only top-level keys vote.
pub(crate) fn Fn(Line:&str) -> bool {
	let Indented = Line.starts_with(' ') || Line.as_bytes().first() == Some(&9);
	let Trimmed = Line.trim_end();
	let Colon = Trimmed.find(':').unwrap_or(0);
	let Key = Trimmed.get(..Colon).unwrap_or("");
	let Shaped = Colon > 0
		&& Colon <= KeyMaximum
		&& !Key.is_empty()
		&& Key.chars().all(|Character| {
			Character.is_ascii_lowercase() || Character.is_ascii_digit() || Character == '-' || Character == '_'
		});

	!Indented && Shaped && !LogMarkers.contains(&Key)
}
