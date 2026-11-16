//! The bytes that leave the proxy.
//!
//! The compressed path used to serialise the transformed body twice - once to
//! measure it for the dev log, once to build the response - and added the saving
//! to the counter at the first of those, which is not the same thing as the
//! bytes that were emitted. Making the final bytes one value means the log, the
//! cache insertion, the response body and the accounting all read the same
//! allocation, and the counter can only be incremented from the body that was
//! actually sent.
//!
//! `Original` is a distinct case, not a zero-saving `Transformed`: an
//! uncompressed body must keep the upstream's exact bytes, and re-serialising a
//! parsed value would change its whitespace.

use bytes::Bytes;

use crate::Proxy::Measurement::Content::Struct::Struct as ContentAccounting;

/// The bytes selected for the client.
pub(crate) enum Enum {
	/// The upstream body, untouched.
	Original(Bytes),
	/// The rewritten body, with what the rewrite saved.
	Transformed { Bytes:Bytes, Accounting:ContentAccounting },
}

impl Enum {
	/// Take the bytes, whichever case this is.
	pub(crate) fn Payload(self) -> Bytes {
		match self {
			Enum::Original(Bytes) => Bytes,
			Enum::Transformed { Bytes, .. } => Bytes,
		}
	}
}
