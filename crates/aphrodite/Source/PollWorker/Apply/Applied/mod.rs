//! The applied poll: one of two outcomes, each owning what it produced.
//!
//! It used to be one struct with a `Transition` field beside an `Accumulated`
//! buffer that was only meaningful for a completion - a "transition plus possibly
//! meaningful fields" pair, which let completion preparation accept a progress
//! result and store an empty buffer under a fresh hash. The two variants now own
//! their own data, so that call cannot be written.
//!
//! There is no third variant for "this poll named no task of mine": that outcome is
//! `Apply::Fn`'s `None`, and it is a different fact - the poll was not consumed at
//! all, rather than consumed with no effect.

pub mod Enum;
pub mod Finished;
pub mod Subject;
