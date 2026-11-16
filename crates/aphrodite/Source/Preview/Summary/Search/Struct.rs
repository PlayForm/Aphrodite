//! The result tally.

use std::collections::BTreeSet;

use crate::Preview::Parse::Search::Hit::Struct::Struct as Record;

/// A parsed result set: how many hits, how many distinct files, and the first hit.
///
/// The file set is a `BTreeSet` because the only fact reported is its size, and a set
/// answers "how many distinct" without a second pass. It is not a ranking: hits are
/// counted as printed, so a file mentioned twice counts once in the file total and twice
/// in the hit total.
pub(crate) struct Struct {
	Hits:usize,
	Files:BTreeSet<String>,
	First:Option<Record>,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Hits:0, Files:BTreeSet::new(), First:None } }

	/// Account for one recognised hit.
	///
	/// The first hit is cloned because it has to outlive the fold, and `get_or_insert_with`
	/// keeps that clone to exactly one; every later hit contributes only its path, which is
	/// moved into the set. It is not a sample: the location shown is the actual first hit in
	/// input order.
	pub(crate) fn Push(&mut self, Hit:Record) {
		self.Hits += 1;

		self.First
			.get_or_insert_with(|| Record { Path:Hit.Path.clone(), Line:Hit.Line.clone() });
		self.Files.insert(Hit.Path);
	}

	pub(crate) fn Hits(&self) -> usize { self.Hits }
	pub(crate) fn Files(&self) -> usize { self.Files.len() }
	pub(crate) fn First(&self) -> Option<&Record> { self.First.as_ref() }
}
