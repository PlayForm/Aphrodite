//! Search shape detector.
//!
//! A search result is a majority of `path:line:` lines, mirroring the shape the
//! old proxy pattern accepted. It is the grep detector minus the file-ish path
//! requirement, and that difference is the only thing separating the two arms.

use crate::Preview::{Input::Struct::Struct as Input, Line::Grep::Search};

/// True when the content looks like search results.
///
/// Two or more `path:line:` lines making up at least half the non-empty lines.
/// It is not the grep detector: a timestamped line is explicitly not a search
/// path, so log output cannot be claimed here.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool { Inp.Majority(Search::Fn, 2) }
