//! Close the open record, if it has one.

use crate::Preview::Parse::GitLog::{Record::Struct::Struct as Record, State::Enum::Enum as State};

/// Emit the open record when it carries a subject, and reset to `Outside`.
///
/// A record still `AwaitingSubject` has no subject - the commit had an empty message,
/// or the input ended inside its header block - so it emits nothing rather than
/// borrowing the next commit's subject. That is the correction: before, the subject
/// search ran past the record boundary and a subject-less commit inherited the
/// following commit's subject. It is not a lossy flush: the commit still counts, it
/// simply contributes no endpoint.
pub(crate) fn Fn(State:&mut State) -> Option<Record> {
	match std::mem::replace(State, State::Outside) {
		State::HaveSubject { Hash, Subject } => Some(Record { Hash, Subject }),
		_ => None,
	}
}
