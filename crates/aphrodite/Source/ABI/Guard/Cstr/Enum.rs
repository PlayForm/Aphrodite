//! What a caller's pointer argument turned out to be.

use std::borrow::Cow;

/// The decoded form of one pointer argument.
///
/// Three variants, because the boundary can learn three different things and
/// collapsing any two of them is what made the previous `Option<String>` lie: a
/// null pointer (the caller passed no argument) and a string over the ingress cap
/// (the caller passed something no export can serve) both arrived as `None`, and
/// every export then had to guess which one it had.
///
/// `Text` borrows whenever the bytes are valid UTF-8 and owns only when the lossy
/// replacement had to build a new string, so an ordinary argument costs no
/// allocation - and none of the three variants can be constructed without the cap
/// having been checked first.
pub(crate) enum Enum<'a> {
	/// The caller passed a null pointer.
	Absent,
	/// The caller's string, at most `Maximum` bytes.
	Text(Cow<'a, str>),
	/// The caller's string is longer than `Maximum`; nothing was copied.
	Oversize,
}
