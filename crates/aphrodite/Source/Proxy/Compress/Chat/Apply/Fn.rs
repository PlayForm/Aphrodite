//! Write the markers the plan produced into the response body.
//!
//! The only mutation of the response is here, and it is driven entirely by the
//! decisions: a choice whose decision has no replacement is not touched at all.
//! `StoredInline` writes nothing, which is what makes it byte-identical to the
//! original on the wire.
//!
//! `tool_calls[].function.arguments` is deliberately out of scope (02-F3): it is
//! JSON the CLIENT feeds into its own tool executor, not model-facing prose, and
//! a prior version that compressed it produced tool calls with
//! `arguments = "<<<CCR:...>>>"` - unparseable JSON that breaks every real
//! OpenAI-tools client. Only `message.content` is a legitimate target.

use crate::Proxy::Compress::Chat::Rewrite::Enum::Enum as ContentRewrite;

/// Write every replacement into the parsed response.
pub(crate) fn Fn(Response:&mut serde_json::Value, Rewrites:&[ContentRewrite]) {
	let Choices = Response.get_mut("choices").and_then(|Choices| Choices.as_array_mut());

	let _ = Choices.map(|Choices| {
		Choices.iter_mut().zip(Rewrites).for_each(|(Choice, Rewrite)| {
			Rewrite.Replacement().into_iter().for_each(|Marker| {
				let _ = Choice
					.get_mut("message")
					.and_then(|Message| Message.get_mut("content"))
					.map(|Slot| *Slot = serde_json::Value::String(Marker.to_owned()));
			});
		});
	});
}
