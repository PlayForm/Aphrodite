//! `Computed::Fn`: the variables derived from the rest of the set.

use std::collections::HashMap;

/// Add `fx`, `Cmx` and the `sig1` default.
///
/// Runs LAST, after the code enrichment, because `sig1` may already have been
/// set from the structure map and must not be overwritten with an empty string.
/// `fx` stays empty on purpose: `fn` is never derivable from content alone, and
/// the pre-Rust engine left it blank rather than guessing.
pub(crate) fn Fn(Vars:&mut HashMap<String, String>) {
	let Command = Vars.get("cmd").cloned().unwrap_or_default();
	let Cmx = match Command.is_empty() {
		true => "?".to_string(),
		false => Command.chars().take(40).collect(),
	};
	Vars.insert("fx".into(), String::new());
	Vars.insert("cmx".into(), Cmx);
	match !Vars.contains_key("sig1") {
		true => {
			Vars.insert("sig1".into(), String::new());
		},
		false => (),
	}
}
