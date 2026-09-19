//! YAML preview arm.

use crate::preview::input::Input;
use crate::preview::line::yaml_key::is_yaml_key_line;

/// YAML preview: top-level key count + first few keys.
/// `[yaml:4 keys 6L | name, version, port, debug]`.
pub(crate) fn build_yaml_preview(inp: &Input<'_>) -> String {
	let keys: Vec<&str> = inp
		.raw
		.lines()
		.filter(|l| is_yaml_key_line(l))
		.filter_map(|l| l.find(':').map(|i| &l[..i]))
		.collect();
	let shown: Vec<&str> = keys.iter().take(5).copied().collect();
	let more = if keys.len() > shown.len() {
		format!(" +{} more", keys.len() - shown.len())
	} else {
		String::new()
	};
	format!("[yaml:{} keys {}L | {}{}]", keys.len(), inp.total, shown.join(", "), more)
}
