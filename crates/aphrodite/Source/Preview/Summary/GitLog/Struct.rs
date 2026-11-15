//! The `git log` endpoints.

use crate::Preview::Parse::GitLog::Record::Struct::Struct as Record;

/// A parsed log: how many commits, and the first and last record.
///
/// Only the two endpoints are retained, so a ten-thousand-commit log costs two records
/// - the count is exact because it is a counter, not the length of a list. It is not a
///   history: a commit in the middle contributes to `Commits` and nothing else.
pub(crate) struct Struct {
	Commits:usize,
	First:Option<Record>,
	Last:Option<Record>,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Commits:0, First:None, Last:None } }

	/// Account for one completed record.
	///
	/// The first record is cloned because it has to survive the loop; every later one
	/// simply replaces `Last`, so exactly one clone happens per log. It is not a
	/// sample: the last record is the actual last one, not a bounded approximation.
	pub(crate) fn Push(&mut self, Completed:Record) {
		self.Commits += 1;
		self.First
			.get_or_insert_with(|| Record { Hash:Completed.Hash.clone(), Subject:Completed.Subject.clone() });
		self.Last = Some(Completed);
	}

	pub(crate) fn Commits(&self) -> usize { self.Commits }
	pub(crate) fn First(&self) -> Option<&Record> { self.First.as_ref() }
	pub(crate) fn Last(&self) -> Option<&Record> { self.Last.as_ref() }
}
