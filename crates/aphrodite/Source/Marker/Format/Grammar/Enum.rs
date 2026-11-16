//! Which grammar a marker is rendered with.

/// The marker grammar in force for this process.
///
/// Two variants, because exactly one of two grammars can be in force: the
/// canonical `<<<CCR:{hash}|{type}|{size}>>>` block, or a `[templates.marker]
/// format` string from the configuration. The `Option<String>` this replaces
/// said the same thing with `None`, which left the fallback nameless - a reader
/// had to know that "no format registered" meant "the canonical grammar" rather
/// than "rendering is broken".
pub(crate) enum Enum {
	/// The shipped grammar, byte-stable across releases.
	Canonical,
	/// A registered `[templates.marker] format` string, substituted with
	/// `{preview}`, `{type}`, `{meta}`, `{center}`, `{center_seg}`, `{hash}`,
	/// `{size}` and `{hint}`.
	Template(String),
}
