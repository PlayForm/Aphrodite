//! Which way the retrieval ratio moves the split threshold.
//!
//! `Enum/` is the movement, `Decide/` is the rule that chooses it. The rule is
//! a pure function of the ratio and the bounds, so it can be read - and changed
//! - without the state that produced the ratio.

pub mod Decide;
pub mod Enum;
