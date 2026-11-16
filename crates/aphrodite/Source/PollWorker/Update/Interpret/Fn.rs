//! What the session does with an applied poll.
//!
//! A completion is stored and announced; a progress poll changes nothing outside the
//! task - the accumulator it grew is the whole effect. There is no arm for "the poll
//! did not apply": `Apply::Fn` answered `None` and this function was never called.
//!
//! The store and the marker are `Interpret::Store`'s job, so this leaf is only the
//! decision, and the decision is the variant match: `Completed` owns the output and the
//! exit code, `Progressed` owns neither, so nothing here can be handed a half-built
//! completion.

use crate::{
	PollWorker::{
		Apply::Applied::Enum::Enum as Outcome,
		Completion::Build::Fn::Fn as Build,
		Update::Interpret::Store::Fn::Fn as Store,
	},
	State::Struct::AphroditeState,
};

/// Store and announce a completed poll, or do nothing.
pub(crate) fn Fn(State:&mut AphroditeState, Outcome:Outcome) {
	let Finished = match Outcome {
		Outcome::Completed(Finished) => Finished,
		// A progress poll's whole effect is inside the task: nothing is stored and
		// nothing is announced. The task it moved is named at debug level so a
		// long-running command's polls are visible in the stream.
		Outcome::Progressed(Subject) => {
			tracing::debug!(Task = %Subject.ID, Index = Subject.Index, "poll worker: progress accumulated");

			return;
		},
	};

	let Index = Finished.Subject.Index;
	let TaskID = Finished.Subject.ID.clone();
	let Completion = Build(Finished);

	tracing::debug!(Task = %TaskID, Hash = %Completion.Stored.Hash, Size = Completion.Stored.Size, "poll worker: completion stored");

	Store(State, Index, Completion);
}
