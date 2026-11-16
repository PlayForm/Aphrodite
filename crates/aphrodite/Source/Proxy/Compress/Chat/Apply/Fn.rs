//! Write the plan into the response body, and keep what it did.
//!
//! The only mutation of the response is here, and it is driven entirely by the
//! decisions: a choice whose decision carries no marker is not touched at all.
//! `StoredInline` writes nothing, which is what makes it byte-identical to the
//! original on the wire.
//!
//! The plan is CONSUMED, not borrowed. Each marker `String` is moved into the
//! JSON slot it replaces, and what comes back is one summary per choice - so the
//! marker text has exactly one owner at every point in the pass, and the outcome
//! keeps the accounting rather than a copy of the bytes the body already holds.
//! The old shape cloned every marker into the body and then kept the original
//! alive in the outcome for nobody to read.
//!
//! `tool_calls[].function.arguments` is deliberately out of scope (02-F3): it is
//! JSON the CLIENT feeds into its own tool executor, not model-facing prose, and
//! a prior version that compressed it produced tool calls with
//! `arguments = "<<<CCR:...>>>"` - unparseable JSON that breaks every real
//! OpenAI-tools client. Only `message.content` is a legitimate target.

use crate::Proxy::Compress::Chat::{
	Outcome::Summary::Enum::Enum as RewriteSummary,
	Rewrite::Enum::Enum as ContentRewrite,
};

/// Write every replacement into `Choices`, and summarise what was done.
pub(crate) fn Fn(Choices:&mut [serde_json::Value], Rewrites:Vec<ContentRewrite>) -> Vec<RewriteSummary> {
	Choices
		.iter_mut()
		.zip(Rewrites)
		.map(|(Choice, Rewrite)| {
			match Rewrite {
				ContentRewrite::Replaced { Content: Marker, Accounting } => {
					// The marker moves into the slot; nothing else holds it. The
					// slot exists whenever the plan produced a `Replaced` - the plan
					// read this same path to decide.
					let _ = Choice
						.get_mut("message")
						.and_then(|Message| Message.get_mut("content"))
						.map(|Slot| *Slot = serde_json::Value::String(Marker));

					RewriteSummary::Replaced { Accounting }
				},
				ContentRewrite::StoredInline => RewriteSummary::StoredInline,
				ContentRewrite::Unchanged(Reason) => RewriteSummary::Unchanged(Reason),
			}
		})
		.collect()
}
