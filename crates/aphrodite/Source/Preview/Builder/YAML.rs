//! YAML preview arm.
//!
//! A YAML document is described by its top-level keys - how many and which. This
//! arm renders those. It is not a YAML parser: anchors, aliases and multi-line
//! scalars are not resolved, and nested keys are deliberately not reported.

use crate::Preview::{Input::Struct::Struct as Input, Line::YAMLKey};

/// Build a preview of YAML content.
///
/// Promises the top-level key count and the first few key names. It is not a
/// value reader: values are never shown, so a config file previews as its key
/// list - which is what the model needs to know which knob to retrieve.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Keys:Vec<&str> = Inp
		.raw
		.lines()
		.filter(|l| YAMLKey::Fn(l))
		.filter_map(|l| l.find(':').map(|i| &l[..i]))
		.collect();
	let Shown:Vec<&str> = Keys.iter().take(5).copied().collect();
	let More = if Keys.len() > Shown.len() {
		format!(" +{} more", Keys.len() - Shown.len())
	} else {
		String::new()
	};
	format!("[yaml:{} keys {}L | {}{}]", Keys.len(), Inp.Total, Shown.join(", "), More)
}
