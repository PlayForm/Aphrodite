//! Recognise JSON, and the Hermes wrapper envelopes inside it.
//!
//! This arm runs before the preview pipeline because that pipeline's JSON
//! detector deliberately excludes tool envelopes from `JSON` (the raw-JSON
//! preview is the payload). Recognising them here keeps a stored envelope
//! routed to the tool_output arm instead of previewing as anonymous JSON.
//!
//! Three outcomes, and the third is why this is a `match` rather than a chain:
//! a body that starts like JSON but does not parse is `text`, not "ask the next
//! classifier" - a truncated or corrupt payload must not be re-examined as
//! something it plainly is not.

/// The envelope/JSON verdict, or `None` when the content is not JSON-shaped.
pub(crate) fn Fn(Content:&str) -> Option<&'static str> {
	(Content.starts_with('{') || Content.starts_with('['))
		.then(|| serde_json::from_str::<serde_json::Value>(Content).ok())
		.map(|Parsed| {
			match Parsed {
				None => "text",
				Some(_) if Content.contains("exit_code") || Content.contains("\"status\"") => "tool_output",
				Some(_) => "json",
			}
		})
}
