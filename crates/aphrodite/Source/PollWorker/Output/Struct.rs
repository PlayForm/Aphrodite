//! The accumulated output of one task, with its bound.
//!
//! The cap is a property of the buffer rather than a check the caller has to
//! remember: `Output::Append` decides what fits and reports what it did, so the
//! buffer is never longer than `Maximum` at any point - not
//! before an append, not after one.
//!
//! Two decisions are explicit here rather than implied by a caller:
//!
//! - **The separator counts against the cap.** The newline between two polls is
//!   part of the buffer the model eventually reads, so room is reserved for it
//!   before any content is measured.
//! - **The prefix is kept and the suffix is dropped.** Truncation keeps the
//!   earliest bytes, which is what the old code kept, and cuts on a `char`
//!   boundary so the kept prefix is always valid UTF-8.

use crate::PollWorker::Output::Const::Maximum;

/// Accumulated poll output, bounded at construction.
#[derive(Debug, Clone)]
pub struct Accumulator {
	/// The kept prefix.
	pub(super) Buffer:String,
	/// The bound this buffer enforces on itself.
	pub(super) Maximum:usize,
}

impl Accumulator {
	/// An empty accumulator at the shipped bound.
	pub(crate) const fn New() -> Self { Self::NewWithin(Maximum) }

	/// An empty accumulator at an explicit bound.
	///
	/// The bound is a field rather than a constant read at the append site, and this is
	/// the only other constructor: a test can pin the cap arithmetic byte by byte at a
	/// size it can reason about (`NewWithin(10)`), while the shipped path always uses
	/// `New`. Nothing else may lower the bound.
	pub(crate) const fn NewWithin(Cap:usize) -> Self { Self { Buffer:String::new(), Maximum:Cap } }

	/// Bytes currently kept - what the live-container surface reports.
	pub(crate) fn Len(&self) -> usize { self.Buffer.len() }

	/// The kept prefix. Test-only: nothing on the turn path reads the buffer as
	/// text, it is moved into the store whole.
	#[cfg(test)]
	pub(crate) fn AsStr(&self) -> &str { &self.Buffer }

	/// Hand the buffer to the caller, leaving an empty accumulator behind.
	pub(crate) fn Take(&mut self) -> String { std::mem::take(&mut self.Buffer) }
}
