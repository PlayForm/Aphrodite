//! Build a `ContentAnalysis` for a family the test names itself.
//!
//! The metadata tests pin one producer at a time, so they must be able to ask for
//! `git` or `error` without depending on the detector's verdict for their
//! fixture - and the line count still has to be the real one, because the
//! producers report it.

use crate::Proxy::Analysis::Struct::Struct as ContentAnalysis;

/// An analysis of `Content` under the given content type.
pub(crate) fn Fn<'a>(Content:&'a str, Ct:&'static str) -> ContentAnalysis<'a> { ContentAnalysis::New(Content, Ct) }
