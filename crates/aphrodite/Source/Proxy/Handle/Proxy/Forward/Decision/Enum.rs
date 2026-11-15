//! The retry verdict.
//!
//! `Stop` is a value, not "the loop happened to end": the interpreter returns on
//! it, so there is no fabricated terminal error seeded before the loop and no
//! boolean beside the result saying whether the last failure was a timeout.

use std::time::Duration;

/// What to do next.
pub(crate) enum Enum {
	/// Wait this long and try again.
	RetryAfter(Duration),
	/// Give up; the error is the caller's to classify.
	Stop,
}
