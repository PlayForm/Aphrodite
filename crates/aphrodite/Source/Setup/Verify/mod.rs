//! Prerequisite checks that run before setup writes anything.
//!
//! `Hermes/` proves the `hermes` CLI is on `PATH` and answering; `SelfHash/`
//! records the BLAKE3 of the binary being installed so the user can identify the
//! artifact afterwards. Both are read-only - nothing here creates, moves or
//! rewrites a file - so a check that fails leaves the previous install untouched
//! and a check that is skipped cannot leave a half-written one.

pub mod Hermes;
pub mod SelfHash;
