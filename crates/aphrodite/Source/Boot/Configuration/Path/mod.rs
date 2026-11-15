//! Deciding which configuration file to read.
//!
//! Three candidates in one lazy chain - explicit override, `./aphrodite.toml`,
//! runtime home - with the answer and its presence carried together so no caller
//! has to remember which way round a boolean went.

pub mod Fn;
pub mod Found;
pub mod Resolved;
