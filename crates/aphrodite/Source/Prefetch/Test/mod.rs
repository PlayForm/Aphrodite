//! Prefetch test suite, split by the leaf each test exercises.
//!
//! `Refuse` is the security suite and is kept as its own file so the guard's
//! proof is never buried among ordinary I/O tests: every arm of
//! `RefuseReason` (`.env`, `auth.json`, `~/.ssh`, the Hermes home, non-file
//! paths) has a test here, and the off-path status contract has its own file
//! so a byte-identity regression cannot hide behind a passing load test.

mod Arrival;
mod Prefetch;
mod Refuse;
mod Status;
