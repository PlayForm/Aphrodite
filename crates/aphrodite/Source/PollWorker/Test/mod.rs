//! Poll-worker test suite, split by the leaf each test exercises.
//!
//! Every area file imports its subject explicitly rather than leaning on a
//! `use super::*` glob: the module tree is the contract, so a test that reaches across
//! areas is visible in its import list.

mod Check;
mod Expire;
mod Insert;
mod Marker;
mod Registry;
mod Should;
mod Status;
mod Update;
