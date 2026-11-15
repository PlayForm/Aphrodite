//! Generic preview arm - the honest fallback.
//!
//! When nothing classified the payload, the preview still has to say something
//! true about it. This arm shows a content hint. It is not a classifier: it
//! never claims to know the shape, it only finds the first line that carries
//! content and samples it fairly.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Text::{FirstMeaningful, SampleLong},
};

/// Build a preview for content of unknown shape.
///
/// Promises a content hint rather than a bare length/byte count, so the model
/// has some signal about what the blob is. It is not a first-line reader: the
/// hint skips structural noise (a lone `{` from pretty-printed JSON) to the
/// first MEANINGFUL line, and samples head+tail for a very long line. Showing
/// the raw first line made a pretty-printed JSON blob preview as `{` - the
/// single most misleading preview the battery ever measured.
pub(crate) fn Fn(TypeStr:&str, Inp:&Input<'_>) -> String {
	let Hint = FirstMeaningful::Fn(Inp.raw)
		.map(|l| SampleLong::Fn(&l))
		.filter(|s| !s.is_empty());
	match Hint {
		Some(h) => format!("[{}:{}L {}B | {}]", TypeStr, Inp.Total, Inp.bytes, h),
		None => format!("[{}:{}L {}B]", TypeStr, Inp.Total, Inp.bytes),
	}
}
