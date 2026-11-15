//! Build one split part's artifacts.

use super::{super::Segment::Struct::Struct as Segment, Decorate::Fn::Fn as Decorate};

/// Build a part's type, hash, preview and marker from its text.
///
/// No session access: everything here is a pure function of the text, which is
/// what lets the whole batch be prepared before any store is touched.
pub(crate) fn Fn(Index:usize, Text:&str) -> Segment {
	let Ct = headroom_core::transforms::content_detector::detect_content_type(Text).content_type;
	let Type = Ct.as_str().to_string();
	let Hash = headroom_core::ccr::compute_key(Text.as_bytes());
	let Builder = crate::Preview::Builder::Entry::Fn(&Type, Text);
	let Preview = Decorate(&Builder, Text).into_owned();
	let Marker = crate::Marker::Format::Entry::CCRMarker(&Hash, &Type, Text.len(), &Preview, None, None, None);

	Segment { Index, Text:Text.to_string(), Type, Hash, Preview, Marker }
}
