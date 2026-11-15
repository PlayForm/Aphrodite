//! The replacement value for a compressed tool/terminal result.
//!
//! This is the invisibility contract in code: the model must see the same kind
//! of thing for every compressed output - a natural CCR marker (plus its
//! preview) that it can retrieve by hash - and never the mechanism's own
//! vocabulary. Returning `null` is a pass-through (leave the original output
//! untouched); returning a string REPLACES it, so a wrong branch here either
//! erases output or leaks internals into the transcript.

use crate::Debug;

/// Map a hook compression result to the value Hermes uses to replace output.
///
/// Hermes only honors a *string* return from `transform_tool_result` /
/// `transform_terminal_output` (first non-None string wins; non-strings pass
/// through). So return the CCR marker string when compression happened, and
/// `null` otherwise to leave the original output untouched. When the hook
/// auto-expanded (`state.auto_expand` on, content within the limit), the
/// resolved `expanded` string is returned instead of the marker - the marker
/// itself stays in the payload and the store, still resolvable via
/// `aphrodite_retrieve`.
pub(crate) fn replacement_from(r:&serde_json::Value, session:&str) -> serde_json::Value {
	// Fine-grained chain split, invisibility contract: the LLM must see
	// exactly what it would for any compressed output - the NATURAL
	// per-segment CCR markers (each segment's own `<<<CCR:hash|type|size>>>`
	// marker, retrievable by its own hash), one per line. Never the
	// `summary` string: it announces the mechanism (`[chain:N segs | ...]`)
	// and stays in the JSON payload for telemetry/metrics only.
	let chain = r
		.get("chain_split")
		.and_then(|v| v.as_bool())
		.unwrap_or(false)
		.then(|| {
			r.get("markers")
				.and_then(|v| v.as_array())
				.into_iter()
				.flatten()
				.filter_map(|m| m.get("marker").and_then(|v| v.as_str()))
				.collect::<Vec<&str>>()
				.join("\n")
		})
		.filter(|joined| !joined.is_empty());

	// Auto-expand (dylib-side consumer): the hook stored the content and
	// resolved it inline, carrying the full text back in `expanded` - return
	// that so no `<<<CCR:...>>>` marker reaches the model. A payload that
	// flags `auto_expanded` without a usable `expanded` string falls through
	// to the marker path below (degrade, never null-replace the output).
	let expanded = r
		.get("auto_expanded")
		.and_then(|v| v.as_bool())
		.unwrap_or(false)
		.then(|| r.get("expanded").and_then(|v| v.as_str()))
		.flatten();

	let marker = r
		.get("compressed")
		.and_then(|v| v.as_bool())
		.unwrap_or(false)
		.then(|| r.get("marker").and_then(|v| v.as_str()))
		.flatten();

	match (chain, expanded, marker) {
		(Some(joined), ..) => serde_json::Value::String(joined),
		(None, Some(expanded), _) => serde_json::Value::String(expanded.to_string()),
		(None, None, Some(marker)) => {
			// Per-session debug (flag file): prepend a `[aphrodite-debug ...]`
			// line before the CCR marker when the toggle is on for this session,
			// so the session sees what the engine did without retrieving. Quiet
			// path (flag off) costs one stat() and returns the marker unchanged.
			let mut out = String::with_capacity(marker.len() + 128);

			if let Some(line) = Debug::Log::Line::Fn::Fn(r, session) {
				out.push_str(&line);
				out.push('\n');
			}

			out.push_str(marker);
			serde_json::Value::String(out)
		},
		(None, None, None) => serde_json::Value::Null,
	}
}
