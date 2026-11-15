//! What one listing entry is.

/// A directory or a file.
///
/// The two are the whole distinction the preview reports, and naming it as a value is
/// what keeps the tally a structural match instead of a boolean test. It is not a file
/// type: symlinks, sockets and devices all count as files, because that is how the old
/// reader counted them and the preview never claimed otherwise.
pub(crate) enum Enum {
	File,
	Directory,
}
