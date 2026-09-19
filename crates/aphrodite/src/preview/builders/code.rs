//! code preview arm (structure map + first signature).

use crate::preview::input::Input;

/// Enrich with the structure map (fns/structs/traits/impls/classes/types +
/// first signature) so the dylib/hook path matches the proxy's preview
/// quality, instead of a bare substring count.
pub(crate) fn build_code_preview(inp: &Input<'_>) -> String {
	let st = crate::struct_extract::extract_code_structure(inp.raw, "");
	let mut parts: Vec<String> = Vec::new();
	for (key, label) in [
		("fns", "fns"),
		("structs", "structs"),
		("traits", "traits"),
		("impls", "impls"),
		("classes", "classes"),
		("types", "types"),
	] {
		if let Some(v) = st.get(key) {
			if !v.is_empty() {
				parts.push(format!("{}{}", v.len(), label));
			}
		}
	}
	let summary = if parts.is_empty() {
		format!("{}fns", inp.raw.matches("fn ").count() + inp.raw.matches("def ").count())
	} else {
		parts.join("|")
	};
	let sig = st
		.get("fns")
		.and_then(|v| v.first())
		.map(|s| format!(" {}", s.chars().take(48).collect::<String>().trim()))
		.unwrap_or_default();
	format!("[code:{}{} {}L]", summary, sig, inp.total)
}
