//! Advance the record machine by one event.

use crate::Preview::Parse::GitLog::{
	Event::Enum::Enum as Event,
	Flush::Fn::Fn as Flush,
	Record::Struct::Struct as Record,
	State::Enum::Enum as State,
};

/// Apply one classified line, returning the record it completes.
///
/// A `Commit` event flushes the open record before opening the new one, so a record can
/// never span two commits. An ordinary line completes the record only when the machine
/// is awaiting a subject AND the line is neither blank nor one of the three headers a
/// log prints between the hash and the message. The header list is exactly the one the
/// forward search used (`Author:`, `Date:`, `Merge:`), so the same lines are skipped -
/// only the boundary changed. It is not a parser of the message: the first admissible
/// line is the subject, and everything after it is ignored.
pub(crate) fn Fn(State:&mut State, Event:&Event<'_>) -> Option<Record> {
	match Event {
		Event::Commit { Hash } => {
			let Flushed = Flush(State);

			*State = State::AwaitingSubject { Hash:(*Hash).to_string() };

			Flushed
		},
		Event::Line { Trimmed } => {
			match std::mem::replace(State, State::Outside) {
				State::AwaitingSubject { Hash } => {
					let Admit = !Trimmed.is_empty()
						&& !Trimmed.starts_with("Author:")
						&& !Trimmed.starts_with("Date:")
						&& !Trimmed.starts_with("Merge:");

					*State =
						match Admit.then(|| State::HaveSubject { Hash:Hash.clone(), Subject:(*Trimmed).to_string() }) {
							Some(Completed) => Completed,
							None => State::AwaitingSubject { Hash },
						};

					None
				},
				Other => {
					*State = Other;

					None
				},
			}
		},
	}
}
