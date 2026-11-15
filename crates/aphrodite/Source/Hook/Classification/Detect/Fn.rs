//! The detector's own answer.

use crate::Hook::Classification::{Source::Enum::Enum as Source, Struct::Struct};

/// Name the content's type from its bytes alone.
///
/// Detection stays in Aphrodite's layer - the vendored classifier is untouched -
/// and this leaf adds nothing to it except the provenance, so a refinement step
/// can tell what it started from.
pub(crate) fn Fn(Content:&str) -> Struct<'_> {
	let Base = headroom_core::transforms::content_detector::detect_content_type(Content).content_type;

	Struct { Type:Base.as_str().to_string(), Content, Source:Source::Detected }
}
