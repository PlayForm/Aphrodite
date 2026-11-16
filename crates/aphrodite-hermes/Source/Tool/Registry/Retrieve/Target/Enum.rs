//! Which of the two mutually-exclusive retrieve arguments was given.

/// What `aphrodite_retrieve` was asked to read.
///
/// The schema states the exclusivity in prose (a top-level union does not survive
/// the provider sanitizers) and `path` wins when both are present - which is
/// exactly the kind of rule that becomes a pair of booleans plus a four-arm match
/// when it is not a type. Here "both" cannot exist: resolution returns one
/// variant, and the handler's match is total over two.
pub(crate) enum Enum<'a> {
	/// A workspace-confined file read; never touches the session lock.
	Path(&'a str),
	/// A marker lookup in the session's store, expanded recursively.
	Hash(&'a str),
}
