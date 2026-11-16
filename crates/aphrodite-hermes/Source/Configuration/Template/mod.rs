//! The config-seeded state template every session is cloned from.
//!
//! Two seeds, chosen at compile time: `Seeded/` loads `aphrodite.toml` and starts
//! the opt-in config watcher, `Plain/` is the documented defaults. Only one of
//! them exists in any build, so neither can drift into being "the real one".

pub mod Fn;

#[cfg(not(test))]
pub mod Seeded;
#[cfg(test)]
pub mod Plain;
