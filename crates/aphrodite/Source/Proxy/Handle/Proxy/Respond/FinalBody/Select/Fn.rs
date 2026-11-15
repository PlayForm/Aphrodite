//! Choose the bytes the client receives.
//!
//! Serialisation happens here and only here: the transformed value is turned
//! into bytes once, and every consumer downstream shares that buffer. A
//! serialisation failure falls back to the original bytes rather than to an
//! empty body - a rewrite that cannot be rendered must not lose the answer.
//!
//! `filter` is what keeps an uncompressed body byte-identical: an `Outcome`
//! that stored content without replacing any (auto-expand, inline storage) is
//! not a reason to re-serialise, because re-serialising a parsed value changes
//! its whitespace.

use bytes::Bytes;

use crate::Proxy::{
	Compress::Chat::Outcome::Struct::Struct as CompressOutcome,
	Handle::Proxy::Respond::FinalBody::Enum::Enum as FinalBody,
};

/// Select the emitted bytes from the upstream body and the rewrite outcome.
pub(crate) fn Fn(Original:&Bytes, Outcome:Option<CompressOutcome>) -> FinalBody {
	Outcome
		.filter(CompressOutcome::Compressed)
		.and_then(|Outcome| {
			serde_json::to_vec(&Outcome.Response).ok().map(|Transformed| {
				FinalBody::Transformed { Bytes:Bytes::from(Transformed), Accounting:Outcome.Accounting() }
			})
		})
		.unwrap_or_else(|| FinalBody::Original(Original.clone()))
}
