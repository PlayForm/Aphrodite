//! The process environment as Home sees it.
//!
//! Two leaves: the env-var reader every resolver above goes through (`Path/`),
//! and the crate-wide test mutex (`Guard/`) that serialises the tests which
//! set those vars. Reading the environment is centralised here so "empty
//! means unset" is decided once - a resolver that reads `std::env` directly
//! would reintroduce the empty-string bug this rule exists to stop.

pub mod Path;

#[cfg(test)]
pub mod Guard;
