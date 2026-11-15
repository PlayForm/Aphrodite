//! `git log` preview arm.
//!
//! Parse the log into its records, aggregate the two endpoints, then render them. It is
//! not a history reader: nothing is resolved, and only the text of the log is read.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render};

/// Build a preview of `git log` output.
///
/// Promises the commit count plus the first and last short hash and subject, so the model
/// knows how far the range reaches and what its endpoints say. A commit with an empty
/// message contributes an empty subject rather than borrowing the next commit's - the
/// record boundary is a boundary.
pub(crate) fn Fn(Inp:&Input<'_>) -> String { Render::GitLog::Fn::Fn(&Parse::GitLog::Fn::Fn(Inp), Inp.Total) }
