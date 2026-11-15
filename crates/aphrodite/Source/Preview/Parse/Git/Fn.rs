//! Aggregate `git status` lines into the status summary.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Parse::Git::Record::Fn::Fn as Record,
	Summary::Git::Struct::Struct as Summary,
};

/// Parse a status listing into its per-code tally and its first few paths.
///
/// One fold, so the tally and the shown paths are filled in the same pass and in
/// input order. It is not a repository query: nothing is stat'ed, so a listing of
/// files that no longer exist summarises the same way.
pub(crate) fn Fn(Inp:&Input<'_>) -> Summary {
	Inp.raw.lines().filter_map(Record).fold(Summary::new(), |mut Summary, Record| {
		Summary.Push(Record);

		Summary
	})
}
