//! The per-turn nudge pass: select the tasks worth speaking about, then render
//! and apply their plans.
//!
//! Three leaves, one per step, because each has its own invariant: `Select` owns
//! eligibility and the two-nudge ceiling, `Render` owns the sentences, and `Fn`
//! owns the push. The pass used to do all three in one backwards loop that
//! formatted inline and stopped by returning an error sentinel from a traversal.

pub mod Fn;
pub mod Plan;
pub mod Polled;
pub mod Render;
pub mod Select;
