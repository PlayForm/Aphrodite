//! The task record, its lifecycle vocabulary and the bounds that govern
//! both.
//!
//! Grouped here because every other poll-worker leaf reads the same three
//! things: what a task is, what state it is in, and how many of them may
//! exist. Nothing in this directory touches `AphroditeState`.

pub mod Const;
pub mod Status;
pub mod Struct;
