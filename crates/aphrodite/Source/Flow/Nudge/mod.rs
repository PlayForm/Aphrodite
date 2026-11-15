//! Ephemeral one-shot nudges: queue, render, expire.
//!
//! Three leaves, one per lifecycle stage, because each has its own invariant:
//! `Push` bounds how many can be queued, `Render` bounds how many can be seen,
//! `Purge` bounds how long one survives. A bug in any of the three changes what
//! the model reads on the next turn, and none of them is reachable from the
//! other two.

pub mod Purge;
pub mod Push;
pub mod Render;

