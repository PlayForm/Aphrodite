//! The plan a terminal transform selected.

use crate::{ChainSplit::Prepared::Struct::Struct as Prepared, Hook::Reason::Enum::Enum as Reason};

/// What the terminal path will do, decided before anything is executed.
///
/// Four plans, and the variants are the whole decision: nothing downstream
/// re-tests a threshold or a flag, so the policy cannot be read two ways.
///
/// `Split` carries the PREPARED batch rather than a marker saying "split me",
/// because whether content is a chain is a property of the content, not of the
/// policy: a split that produces fewer than two parts is not a split at all, and
/// a plan that could not be executed would push that check back into the
/// interpreter. Everything in the batch is a pure function of the content, so
/// selection still mutates nothing.
pub enum Enum {
	/// Return the content untouched, for the stated reason.
	Passthrough(Reason),
	/// Store each chain segment as its own marker.
	Split(Prepared),
	/// Store the content as a live container and emit an instant marker.
	Live,
	/// Store the content and emit one ordinary marker.
	Compress,
}
