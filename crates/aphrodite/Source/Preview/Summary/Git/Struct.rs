//! The `git status` tally.

use std::collections::BTreeMap;

use crate::Preview::Parse::Git::Record::Struct::Struct as Record;

/// Paths one status preview shows: three.
const PathsShown:usize = 3;

/// The wire order of the status codes, and the label each one renders as.
///
/// A `?` renders as `??` because that is how git spells an untracked file in its own
/// porcelain output; every other code renders as itself. Keeping the order here (and
/// not in the renderer) is what makes the tally's presentation stable across a change
/// of format.
const Order:[(char, &str); 9] = [
	('M', "M"),
	('A', "A"),
	('D', "D"),
	('R', "R"),
	('C', "C"),
	('U', "U"),
	('T', "T"),
	('?', "??"),
	('!', "!"),
];

/// A parsed status listing: how many of each code, and the first few paths.
///
/// `Total` counts every recognised status line, which is not `Codes`'s sum - a code
/// outside the canonical order still counts in the total and in the `+N more`
/// arithmetic, it just never gets a slot in the tally string. It is not a diff
/// summary: nothing about the content of a change is retained.
pub(crate) struct Struct {
	Codes:BTreeMap<char, usize>,
	Paths:Vec<String>,
	Total:usize,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Codes:BTreeMap::new(), Paths:Vec::new(), Total:0 } }

	/// Account for one recognised status line.
	///
	/// The tally takes every record; the path list takes only the first three that
	/// carry a path, so the memory the summary holds is bounded by the constant above
	/// rather than by the size of the listing. It is not a filter on the record: an
	/// empty path still moves the tally and the total.
	pub(crate) fn Push(&mut self, Record:Record) {
		*self.Codes.entry(Record.Code).or_insert(0) += 1;
		self.Total += 1;

		let Retain = self.Paths.len() < PathsShown && !Record.Path.is_empty();

		self.Paths.extend(Retain.then_some(Record.Path));
	}

	/// The tally in the canonical code order, as `(label, count)` pairs.
	pub(crate) fn Codes(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
		Order
			.into_iter()
			.filter_map(|(Code, Label)| self.Codes.get(&Code).map(|Count| (Label, *Count)))
	}

	/// The first few paths, in input order.
	pub(crate) fn Paths(&self) -> &[String] { &self.Paths }

	/// Every recognised status line, including codes the tally does not print.
	pub(crate) fn Total(&self) -> usize { self.Total }
}
