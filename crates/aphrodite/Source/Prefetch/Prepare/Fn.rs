//! Turn a read file into a prepared one, with no state access.

use crate::{Prefetch::Prepared::Struct::Prepared, State::Identity::Content::Struct::Struct as Digest};

/// Classify, hash and preview `Content`, and record its length.
///
/// This is the whole of what used to happen inside the insertion pass. It takes no
/// state, so it can run before the lock and its cost is proportional to the bytes it
/// was handed and nothing else - which is what lets the lock cover storage and history
/// only.
pub(crate) fn Fn(Path:String, Content:String) -> Prepared {
	let Type = headroom_core::transforms::content_detector::detect_content_type(&Content)
		.content_type
		.as_str()
		.to_string();
	let Hash = Digest::from(headroom_core::ccr::compute_key(Content.as_bytes()));
	let Preview = crate::Preview::Builder::Entry::Fn(&Type, &Content);
	let Size = Content.len();

	Prepared { Path, Content, Hash, Type, Preview, Size }
}
