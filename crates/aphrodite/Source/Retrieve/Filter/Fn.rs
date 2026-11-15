//! Keep only the lines of a payload that match a query.

use std::borrow::Cow;

use crate::Resolve::Query::{Fold, Match};

/// Keep only lines of `Content` matching `Query` (case-insensitive substring),
/// or return `Content` unchanged if `Query` is `None`/empty.
///
/// The query is truncated to 512 chars BEFORE it is lowercased, so the cap bounds
/// the filter itself and not just a display string; it is cut on a char boundary,
/// never a raw byte slice, so a multi-byte query cannot panic (report 05 F2).
///
/// Returns a `Cow` because the pass-through case - the common one - used to
/// allocate a copy of the whole document only to hand it back unchanged. Nothing
/// downstream can tell the difference, and the byte round-trip the endpoint
/// guarantees is unaffected: a borrowed answer IS the stored content.
///
/// The line scan is [`Match::Fn::Fn`], shared with the resolver's filter. This
/// caller folds ASCII only, which is the fold its 512-character cap was written
/// against.
pub fn Fn<'a>(Content:&'a str, Query:Option<&str>) -> Cow<'a, str> {
	let Some(Query) = Query.filter(|Query| !Query.is_empty()) else {
		return Cow::Borrowed(Content);
	};

	let Query = crate::StructExtract::Format::Floor::Fn(Query, 512);
	let Matching = Match::Fn::Fn(Content, &Query.to_ascii_lowercase(), &Fold::Enum::Enum::ASCII);

	match Matching.as_slice() {
		[] => Cow::Owned(format!("[no lines matching {:?} in {} lines]", Query, Content.lines().count())),
		Matching => Cow::Owned(Matching.join("\n")),
	}
}
