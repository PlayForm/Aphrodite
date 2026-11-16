//! One builtin directive.

/// A directive as the core crate embeds it.
///
/// `Content` is the `.md` source compiled into the core crate with
/// `include_str!`, unchanged: this value carries data, and nothing here parses,
/// normalises or re-renders it.
pub(crate) struct Struct {
	/// The directive's name - the file stem the core loader reads back.
	pub(crate) Name:String,
	/// The embedded prose, byte-for-byte.
	pub(crate) Content:String,
}
