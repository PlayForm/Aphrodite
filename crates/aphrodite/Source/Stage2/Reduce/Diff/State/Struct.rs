//! The open file section and its hunk count.

use crate::Stage2::Reduce::Diff::{Event::Enum::Enum as Event, Summary::Struct::Struct as Summary};

/// The file section currently being accumulated.
///
/// Two fields that always move together, and one operation - "a new file header arrived" - that
/// used to be spelled in two places (once inside the loop, once after it). Both the transition and
/// the end-of-input flush return the completed section, so the boundary rule exists once.
///
/// It is not a diff parser: it counts `@@` headers and remembers the header line, nothing else.
pub(crate) struct Struct {
	Current:Option<String>,
	Hunks:usize,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Current:None, Hunks:0 } }

	/// Apply one event, returning the file section it completes.
	///
	/// A `File` event closes whatever section was open and starts a new one, so a section can never
	/// span two files. A `Hunk` event increments the count unconditionally, which is what preserves
	/// the previous handling of a hunk that appears BEFORE the first file header: it is counted,
	/// and then discarded when the first header resets the counter without ever having had a
	/// section to flush.
	///
	/// It is not a validator: a file with no hunks is a real section with a count of zero.
	pub(crate) fn Apply(&mut self, Event:Event<'_>) -> Option<Summary> {
		match Event {
			Event::File { Header } => {
				let Completed = self.Current.take().map(|Current| Summary { Header:Current, Hunks:self.Hunks });

				self.Current = Some(Header.to_string());
				self.Hunks = 0;

				Completed
			},
			Event::Hunk => {
				self.Hunks += 1;

				None
			},
			Event::Other => None,
		}
	}

	/// Close the last file section at end of input.
	///
	/// This is the one boundary the per-event transition cannot see, and it is explicit rather than
	/// duplicated: before, the same three lines appeared both inside the loop and after it.
	pub(crate) fn Flush(&mut self) -> Option<Summary> {
		self.Current.take().map(|Header| Summary { Header, Hunks:self.Hunks })
	}
}
