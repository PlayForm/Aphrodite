//! yaml shape detector.

use crate::preview::{input::Input, line::yaml_key::is_yaml_key_line};

/// ≥3 top-level lowercase-key lines (`name: webapp`). Log-marker keys
/// (`error:`/`warning:`/...) are excluded so a compiler log cannot be
/// mis-tagged as yaml.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let yaml_keys = inp.raw.lines().filter(|l| is_yaml_key_line(l)).count();
	yaml_keys >= 3
}
