//! Aggregate listing entries into the listing summary.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Parse::Listing::{Entry::Fn::Fn as Entry, Keep::Fn::Fn as Keep},
	Summary::Listing::Struct::Struct as Summary,
};

/// Parse a directory listing into its file and directory counts and its extensions.
///
/// One fold over the entries the keep rule admits, so the counts and the extension
/// tally are filled in the same pass. It is not a tree: nesting is flattened, so a
/// listing of directories and a deep tree report the same two counts.
pub(crate) fn Fn(Inp:&Input<'_>) -> Summary {
	Inp.raw
		.lines()
		.filter(|Line| Keep(Line))
		.map(Entry)
		.fold(Summary::new(), |mut Summary, Entry| {
			Summary.Push(Entry);

			Summary
		})
}
