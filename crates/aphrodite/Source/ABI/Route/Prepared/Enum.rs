//! A command, with or without its external effects already performed.

use crate::{ABI::Route::Command::Enum::Enum as Command, Prefetch::Outcome::Enum::Enum as Outcome};

/// A command ready to run against a session.
///
/// The two variants are the two states a command can be in when the session lock
/// is taken: it either needed no external effect, or it did and the effect has
/// already happened. `prefetch` is the only command in the second state, and
/// carrying its outcomes in the variant is what removes the `RefCell` that used
/// to smuggle them past the borrow of the args: the match arm that consumes them
/// is now the arm that owns them, so there is nothing to `take` and nothing to
/// `expect`.
pub(crate) enum Enum<'a> {
	/// No external effect to perform.
	Ready(Command<'a>),
	/// The prefetch files were read from disk before the session lock was taken.
	Prefetch { Outcomes:Vec<(String, Outcome)> },
}
