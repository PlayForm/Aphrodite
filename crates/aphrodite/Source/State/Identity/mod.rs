//! The three identities the state's records are keyed by.
//!
//! They were all `String`, which made them interchangeable: a command identity
//! could be stored in the execution slot, an execution identity compared against a
//! content hash, and nothing objected. Each is now its own type with one
//! constructor and no `Deref`, so a boundary that expects one cannot silently
//! absorb another - and a `compile_fail` fixture on each type is the proof.
//!
//! - `Command/` - the CCR hash of a command line: the poll worker's deduplication
//!   key, shared on purpose by two simultaneous runs of one command.
//! - `Execution/` - the wire's `session_id`: the process one poll names, learned
//!   from the first poll that reports it.
//! - `Content/` - the CCR hash of stored content: what a marker resolves.
//!
//! None of the three is display text. A command LINE is display text and stays a
//! `String` (`BgTask::command`); a preview is display text and stays a `String`.
//! The distinction is the point: an identity is compared, a label is read.

pub mod Command;
pub mod Content;
pub mod Execution;

#[cfg(test)]
mod Test;
