//! The decision for one message, and the marker it wants written.
//!
//! This replaces `DidCompress` plus an in-place mutation of the response JSON.
//! The three cases are the three things that can actually happen, and the middle
//! one is why a boolean was wrong: `StoredInline` means the content is now
//! retrievable while the wire response is byte-identical, which a "did we
//! compress" flag reports as false and a reader then treats as "nothing
//! happened".
//!
//! `Replaced` owns the marker TEXT, because writing it into the body is the last
//! thing that happens to it: the applier consumes this value and moves the
//! `String` into the response slot. The outcome keeps a `Summary` instead of a
//! second copy of the same bytes.
//!
//! `Unchanged` carries a static diagnostic reason. It is text for the dev log,
//! never branched on - the decision itself is the variant, not the reason - so a
//! `&'static str` is the honest type here rather than a one-variant-per-word
//! enum nobody matches.

use crate::Proxy::Measurement::Content::Struct::Struct as ContentAccounting;

/// What to do with one message's content.
pub(crate) enum Enum {
	/// Left alone; the string says why, for the dev log.
	Unchanged(&'static str),
	/// Stored so a retrieval finds it, with the wire response untouched.
	StoredInline,
	/// Replace the content with this marker, which saves this much.
	Replaced { Content:String, Accounting:ContentAccounting },
}
