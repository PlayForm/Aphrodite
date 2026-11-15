//! The hook call.
//!
//! `Fn/` is the fold over the loaded scripts, `Chain/` is the accumulator it threads
//! (and the two leaves that step and finish it), and `Scope/` is the decode of the
//! payload into what a script sees. The split turns the call path into
//! payload → scope → chain → answer, with no mutable pair threaded by hand.

pub mod Chain;
pub mod Fn;
pub mod Scope;
