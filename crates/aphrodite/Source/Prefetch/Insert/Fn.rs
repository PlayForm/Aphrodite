//! The classify-and-store pass: the only part of prefetch that needs the lock.
//!
//! Four stages, and each owns one thing: `Read` did the I/O, `Prepare` classifies and
//! derives identities from the bytes (no state), `Apply` stores and records (no I/O),
//! and `Report` renders the answer (no mutation). This leaf is the composition - it
//! walks the outcomes in the order they were read, so the response's order, the store's
//! eviction order and the file list all keep the caller's order.
//!
//! What is left inside the borrowed window is exactly the mutation: classification,
//! hashing and preview construction happen in `Prepare`, which takes no state at all, so
//! the session is not held for a batch's analysis.

use crate::{
	Prefetch::{
		Apply::Fn::Fn as Apply,
		Clock::Fn::UnixNow,
		Outcome::Enum::ReadOutcome,
		Prepare::Fn::Fn as Prepare,
		Read::Const::MaximumPrefetchSize,
		Record::Enum::Record,
		Report::Fn::Fn as Report,
	},
	State::Struct::AphroditeState,
};

/// Classify and store already-read file contents into `State`. Pure state mutation +
/// JSON assembly - no I/O, so this is the only part that needs the lock.
///
/// One clock read stamps the whole batch: every entry this call loads shares one
/// arrival second, so a reader comparing two entries of the same prefetch never sees
/// them arrive in different seconds because the loop took that long.
pub fn InsertOutcomes(State:&mut AphroditeState, Outcomes:Vec<(String, ReadOutcome)>) -> serde_json::Value {
	let ObservedAtUnix = UnixNow();

	let Records:Vec<Record> = Outcomes
		.into_iter()
		.map(|(Path, Outcome)| {
			match Outcome {
				ReadOutcome::Loaded { Content } => Apply(State, Prepare(Path, Content), ObservedAtUnix),
				ReadOutcome::SkippedSize { Size } => Record::Skipped { Path, Size, Maximum:MaximumPrefetchSize },
				ReadOutcome::Refused { Reason } => Record::Refused { Path, Reason },
				ReadOutcome::Missing | ReadOutcome::Error => Record::Missing { Path },
			}
		})
		.collect();

	Report(Records, State.InlineStoreBytes(), State.InlineStoreByteBudget())
}
