//! Render the `git log` endpoints.

use crate::Preview::Summary::GitLog::Struct::Struct as Summary;
use super::Line::Fn::Fn as Line;

/// Render a log summary as `[gitlog:<n> commits | <first> → <last>]`.
///
/// One commit renders without the arrow, because there is no range to describe, and no
/// commit renders as a bare line count. It is not a history: the endpoints are the whole
/// payload, which is why the summary keeps only them.
pub(crate) fn Fn(Summary:&Summary, Total:usize) -> String {
	match (Summary.Commits(), Summary.First(), Summary.Last()) {
		(0, ..) => format!("[gitlog:{Total}L]"),
		(1, Some(First), _) => format!("[gitlog:1 commit | {}]", Line(First)),
		(Commits, Some(First), Some(Last)) => {
			format!("[gitlog:{Commits} commits | {} → {}]", Line(First), Line(Last))
		},
		(Commits, ..) => format!("[gitlog:{Commits} commits]"),
	}
}
