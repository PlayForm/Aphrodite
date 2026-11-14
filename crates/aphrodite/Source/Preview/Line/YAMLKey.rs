//! Top-level YAML key-line predicate (`name: webapp`).

/// True for a top-level YAML key line (`name: webapp`): a lowercase
/// identifier key, no leading indent, non-empty value side allowed. Log-marker
/// keys are excluded so compiler logs are never mis-tagged as yaml.
pub(crate) fn is_yaml_key_line(line:&str) -> bool {
	if line.starts_with(' ') || line.as_bytes().first() == Some(&9) {
		return false;
	}
	let t = line.trim_end();
	let idx = match t.find(':') {
		Some(i) => i,
		None => return false,
	};
	if idx == 0 || idx > 64 {
		return false;
	}
	let key = &t[..idx];
	if !key
		.chars()
		.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
	{
		return false;
	}
	// Exclude log-marker keys (`error:`/`warning:`/...) so a compiler log is
	// never mis-tagged as yaml. `debug:`/`trace:` are legitimate yaml keys.
	!matches!(key, "error" | "warning" | "note" | "info" | "warn")
}
