//! Narrowing a resolved body to the lines a `query` matched.

/// The body to return: narrowed to the matching lines when a query was given.
///
/// One leaf for both sources because the narrowing is identical for a file read
/// and a marker expansion - the two arms used to carry their own copy, and a
/// change to the filter would have had to land twice.
///
/// An empty query is "no narrowing", not "match nothing": it is the argument's
/// absent value, and returning an empty body for it would turn a plain retrieve
/// into a guaranteed miss.
pub(crate) fn Fn(Body:String, Query:&str) -> String {
	match Query.is_empty() {
		true => Body,
		false => aphrodite::Resolve::Filter::Fn(&Body, Query),
	}
}
