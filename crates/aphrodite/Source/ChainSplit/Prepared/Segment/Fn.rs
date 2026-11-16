//! Build one split part's artifacts.

use super::{super::Segment::Struct::Struct as Segment, Decorate::Fn::Fn as Decorate};

/// Build a part's type, hash, preview and marker from its text.
///
/// `Text` is taken BY VALUE and retained: the parser already allocated it, and
/// the batch is the last owner before the store, so copying it here would be a
/// second full copy of every segment of every chained command. It is borrowed
/// for classification, hashing, previewing and marker rendering - every
/// artifact is derived from the same bytes - and then moved into the segment.
///
/// No session access: everything here is a pure function of the text and the
/// process-wide rendering configuration, which is what lets the whole batch be
/// prepared before any store is touched.
pub(crate) fn Fn(Index:usize, Text:String) -> Segment {
	let Ct = headroom_core::transforms::content_detector::detect_content_type(Text.as_str()).content_type;
	let Type = Ct.as_str().to_string();
	let Hash = headroom_core::ccr::compute_key(Text.as_bytes());
	let Builder = crate::Preview::Builder::Entry::Fn(&Type, Text.as_str());
	let Preview = Decorate(&Builder, Text.as_str()).into_owned();
	let Marker = crate::Marker::Format::Entry::Fn(&Hash, &Type, Text.len(), &Preview, None, None, None);

	Segment { Index, Text, Type, Hash, Preview, Marker }
}
