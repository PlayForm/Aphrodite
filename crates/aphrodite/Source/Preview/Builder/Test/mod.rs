//! Test-output preview arm, split into the three summary shapes it reads.
//!
//! A test run is described by three counts and by the name of the first failure, and the
//! counts can come from three different summary shapes: cargo's `test result:` lines,
//! pytest's `N passed, M failed` lines, or a bare log of `test name ... ok` lines. Each
//! shape is its own parser here because they are genuinely different acceptance rules, and
//! the entry point tries them in that order - the first shape that finds a tally wins, and a
//! later one never runs once an earlier one has answered.

pub mod Bare;
pub mod Cargo;
pub mod Duration;
pub mod Entry;
pub mod Failure;
pub mod Pytest;
pub mod Totals;
