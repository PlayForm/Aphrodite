//! Hash extraction from one marker.
//!
//! A marker body is `hash|type|size`, and only the first field is the key the
//! store answers to. This module owns that split. It is not a validator: an empty
//! hash still parses, because the resolver's job is to look up what was written,
//! not to refuse it - a marker with an empty hash simply misses the store.

use super::{Prefix, Suffix};

/// Parse a CCR marker string to extract the hash.
///
/// Marker format: `<<<CCR:hash|type|size>>>`. Everything from the first `|` on is
/// discarded; the hash is the whole body when there is no pipe. `None` means the
/// text is not a complete marker - the delimiters are missing or out of order.
///
/// It is not a sanitizer and it does not trim: a hash with stray whitespace
/// parses as-is and then misses the store, which is the caller's argument bug to
/// fix at its own boundary (`Marker::normalize_hash`).
pub(crate) fn Fn(Marker:&str) -> Option<String> {
	let Inner = Marker.strip_prefix(Prefix::Const)?.strip_suffix(Suffix::Const)?;
	Inner.split('|').next().map(|h| h.to_string())
}
