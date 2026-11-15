//! `RuntimeHome`: the user-data folder with a degraded fallback.

use std::path::PathBuf;

use super::Opt::Fn::Fn as RuntimeHomeOpt;

/// Runtime home (the user-data folder that holds `aphrodite.toml`,
/// `binaries/`, `directives/`, `ccr.db`, logs).
///
/// Degrades to `.` (never fails) - [`Opt::Fn`] distinguishes the degraded case
/// for callers that need it. The `.` fallback is not a usable home: it exists
/// so a misconfigured process reads a relative path and fails on the first
/// file operation instead of panicking during resolution.
pub fn Fn() -> PathBuf { RuntimeHomeOpt().unwrap_or_else(|| PathBuf::from(".")) }
