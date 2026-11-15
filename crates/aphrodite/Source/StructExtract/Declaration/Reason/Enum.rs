//! Why a declaration could not be read.

/// The two ways a line can look like a declaration and still not yield a signature.
///
/// Both used to be collapsed: a `fn` prefix with no parenthesis fell into an empty `None` arm and
/// vanished, and a declaration with no readable name became `"?"`, which put a question mark into the
/// preview as though it were an identifier. Naming the reasons is the recovery policy: the signature
/// is still not produced, it just says why instead of inventing a name.
pub(crate) enum Enum {
	/// A declaration keyword with no parameter list.
	NoParameters,
	/// A declaration whose name field was empty.
	NoName,
}
