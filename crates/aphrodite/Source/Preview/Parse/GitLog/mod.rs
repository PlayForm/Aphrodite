//! `git log` record parsing - a record-boundary state machine.
//!
//! A log is a sequence of records that each open with a `commit <hash>` header. The
//! subject of a record is the first line after its headers that carries text - and the
//! search for it must stop at the next `commit` header, because a commit with an empty
//! message has no subject and inheriting the next commit's subject names the wrong
//! change in a preview whose whole content is its two endpoints.

pub mod Event;
pub mod Flush;
pub mod Fn;
pub mod Record;
pub mod State;
pub mod Transition;
