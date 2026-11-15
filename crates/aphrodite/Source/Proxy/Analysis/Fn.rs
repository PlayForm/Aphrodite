//! Analyse content once, for every renderer downstream.

use crate::Proxy::Analysis::{Classify::Fn::Fn as ClassifyContent, Struct::Struct as ContentAnalysis};

/// Classify `Content` and count its lines, once.
pub(crate) fn Fn(Content:&str) -> ContentAnalysis<'_> {
	ContentAnalysis { Content, Ct:ClassifyContent(Content), Lines:Content.lines().count() }
}
