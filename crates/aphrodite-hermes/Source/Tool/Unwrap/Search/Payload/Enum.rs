//! What a search envelope actually carries.

/// The two shapes a genuine search envelope can have.
///
/// The variants exist because the payloads are not interchangeable: rows carry
/// the real `path:line:content` text and are typed `search`, while a count-only
/// result carries the total and the truncated flag and is typed `text`. A single
/// `String` return would have to encode which one it was in the string itself.
pub(crate) enum Enum {
	/// One grep-style `path:line:content` line per real match.
	Rows(Vec<String>),
	/// No per-match content: the REAL total, and whether the search truncated.
	Total { Count:u64, Truncated:bool },
}
