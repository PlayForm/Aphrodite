//! `git status` preview arm.
//!
//! Parse the listing into a per-code tally, then render that tally. It is not a
//! repository query: the text git printed is all that is read, so nothing is stat'ed and
//! an empty listing previews as a bare line count.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render};

/// Build a preview of `git status` output.
///
/// Promises a per-code tally (`M`, `A`, `D`, `??`) and the first few paths, so the model
/// learns how much is staged and which files are involved. It is not a diff: the content
/// of a change is never read.
pub(crate) fn Fn(Inp:&Input<'_>) -> String { Render::Git::Fn::Fn(&Parse::Git::Fn::Fn(Inp), Inp.Total) }
