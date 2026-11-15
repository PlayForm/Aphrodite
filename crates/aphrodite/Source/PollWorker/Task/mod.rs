//! The task record, its lifecycle vocabulary and the bounds that govern both.
//!
//! Grouped here because every other poll-worker leaf reads the same things: what a
//! task is, what state it is in, how expendable that state is, and how many of
//! them may exist. Nothing in this directory touches `AphroditeState`.

pub mod Const;
pub mod New;
pub mod Status;
pub mod Struct;
