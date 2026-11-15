//! A resolved type, the content it was resolved from, and how.

use super::Source::Enum::Enum as Source;

/// The reported `type`, the content the preview is built from, and the source.
///
/// `Content` is not always the transformed payload: a caller that has found the
/// real payload underneath a wrapper the core classifier cannot see through
/// passes it as the classify pair, and the preview is built from that while the
/// ORIGINAL bytes are what get hashed and stored. Keeping both in one value is
/// what stops the two paths forking on which string they classify.
#[derive(Debug, Clone)]
pub struct Struct<'a> {
	/// The content type reported in the payload and the marker.
	pub Type:String,
	/// The content the preview is rendered from.
	pub Content:&'a str,
	/// Which step decided `Type`.
	pub Source:Source,
}
