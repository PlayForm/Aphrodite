//! The decision for one message, and the accounting it carries.
//!
//! This replaces `DidCompress` plus an in-place mutation of the response JSON.
//! The three cases are the three things that can actually happen, and the middle
//! one is why a boolean was wrong: `StoredInline` means the content is now
//! retrievable while the wire response is byte-identical, which a "did we
//! compress" flag reports as false and a reader then treats as "nothing
//! happened".
//!
//! `Unchanged` carries a static diagnostic reason. It is text for the dev log,
//! never branched on - the decision itself is the variant, not the reason - so a
//! `&'static str` is the honest type here rather than a one-variant-per-word
//! enum nobody matches.

use crate::Proxy::Measurement::Wire::Struct::Struct as WireAccounting;

/// What happened to one message's content.
pub(crate) enum Enum {
	/// Left alone; the string says why, for the dev log.
	Unchanged(&'static str),
	/// Stored so a retrieval finds it, with the wire response untouched.
	StoredInline,
	/// Replaced by a marker, with what the replacement saved.
	Replaced { Content:String, Accounting:WireAccounting },
}

impl Enum {
	/// The marker text, when this message was replaced.
	pub(crate) fn Replacement(&self) -> Option<&str> {
		match self {
			Enum::Replaced { Content, .. } => Some(Content),
			_ => None,
		}
	}

	/// A one-word name for the dev log.
	pub(crate) fn Summary(&self) -> &'static str {
		match self {
			Enum::Unchanged(Reason) => Reason,
			Enum::StoredInline => "stored_inline",
			Enum::Replaced { .. } => "replaced",
		}
	}
}
