//! Unit tests for directive provisioning, split by area.
//!
//! The three concerns provisioning has - WHERE it lands, WHAT it writes, and
//! the FFI surface that drives both - fail for different reasons, so they are
//! tested separately. Shared fixtures (env guard, scratch home, expected file
//! set) live in their own leaves rather than inline here, so a facade carries
//! no logic.

mod Expected;
mod FFI;
mod Guard;
mod Materialize;
mod Resolve;
mod Temp;
