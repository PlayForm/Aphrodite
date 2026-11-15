//! Where a retrieve reads its content from.

/// The one source a retrieve resolves against.
///
/// The request used to answer this with a pair of emptiness tests at the call
/// site, so "the caller sent nothing" and "the caller sent an empty string" were
/// the same `None`, and a handler that also accepted a path had to spell the
/// decision as `(path.is_empty(), hash.is_empty())` - four combinations of which
/// one is meaningful.
///
/// `Hash` can no longer hold an empty string: the parser normalises and filters
/// before it constructs one, so the type carries the answer. `Absent` is the
/// single refusal, and it is a variant rather than a boolean because the caller
/// matches it to choose the response it sends.
///
/// A `Path` source belongs to the tool layer's retrieve
/// (`crates/aphrodite-hermes/Source/Tool/Registry/Retrieve/Fn.rs`), which is
/// outside this crate and keeps its own pair of tests today; when that site moves
/// onto this type it adds the variant here and the disk read becomes the second
/// arm of the handler's match.
pub enum Enum {
	/// A normalised, non-empty marker hash - the session-store path.
	Hash(String),
	/// No usable hash was supplied.
	Absent,
}
