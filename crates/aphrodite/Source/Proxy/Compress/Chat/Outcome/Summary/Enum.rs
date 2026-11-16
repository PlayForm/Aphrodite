//! What one choice's content became, after the plan was applied.
//!
//! The plan owns the marker text, because writing it into the body is the last
//! thing that happens to it - so the apply consumes the plan and moves the marker
//! into the response. Keeping the same `String` in the outcome as well would be a
//! second owner of bytes the body already holds, and a place for the two copies
//! to disagree about what was emitted.
//!
//! What the responder actually needs afterwards is the kind and, for a
//! replacement, the accounting. `StoredInline` stays distinct from `Unchanged`
//! for the reason a boolean could never express: the content is retrievable while
//! the wire body is byte-identical.

use crate::Proxy::Measurement::Content::Struct::Struct as ContentAccounting;

/// What happened to one message's content.
pub(crate) enum Enum {
	/// Left alone; the string says why, for the dev log.
	Unchanged(&'static str),
	/// Stored so a retrieval finds it, with the wire response untouched.
	StoredInline,
	/// Replaced by a marker the response body now carries, with what that saved.
	Replaced { Accounting:ContentAccounting },
}

impl Enum {
	/// A one-word name for the dev log.
	pub(crate) fn Summary(&self) -> &'static str {
		match self {
			Enum::Unchanged(Reason) => Reason,
			Enum::StoredInline => "stored_inline",
			Enum::Replaced { .. } => "replaced",
		}
	}

	/// Did this choice change anything at all - replaced, or stored inline with
	/// the wire body untouched?
	pub(crate) fn Changed(&self) -> bool { !matches!(self, Enum::Unchanged(_)) }

	/// The accounting, when this choice replaced its content with a marker.
	pub(crate) fn Accounting(&self) -> Option<ContentAccounting> {
		match self {
			Enum::Replaced { Accounting } => Some(*Accounting),
			_ => None,
		}
	}
}
