//! Read the request's source.

use super::super::Enum;

/// Decide where a retrieve reads from, out of the request's raw hash field.
///
/// The normaliser runs BEFORE the emptiness test, and that order is the whole
/// point: a model sometimes echoes the entire marker body back as the hash
/// argument, and the store's lookups are exact-match only, so stripping the
/// `|type|size` suffix is what makes such a request resolve. A test placed first
/// would call a body that is entirely `|`-suffixed "absent".
///
/// Everything after the normaliser is a chain: normalise, drop the empty answer,
/// take ownership of what survives. `map_or` turns the two surviving cases into
/// the enum, so no caller can hold a `Hash` that is empty.
pub fn Fn(Hash:Option<&str>) -> Enum::Enum {
	Hash.map(crate::Marker::Parse::Normalize::NormalizeHash)
		.filter(|Normalized| !Normalized.is_empty())
		.map(str::to_owned)
		.map_or(Enum::Enum::Absent, Enum::Enum::Hash)
}
