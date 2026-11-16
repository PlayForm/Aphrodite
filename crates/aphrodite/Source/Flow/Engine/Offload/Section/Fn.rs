//! Wrap the engine's prompt into the injected section text.

/// Wrap the engine's offload prompt into the injected section text.
///
/// `None` or an empty prompt injects nothing (empty string), so an engine that
/// archived nothing cannot produce an empty `[engine-offload: ]` panel.
pub(crate) fn Fn(Prompt:Option<String>) -> String {
	Prompt
		.filter(|Text| !Text.is_empty())
		.map(|Text| format!("[engine-offload: {Text}]"))
		.unwrap_or_default()
}
