//! `HermesHome`: the Hermes home with a degraded fallback.

use std::path::PathBuf;

use super::Opt::Fn::Fn as HermesHomeOpt;

/// Hermes home: `$HERMES_HOME` when set (expanded), else the platform user
/// home + `.hermes`. Mirrors the plugin shim's `_hermes_home()` and
/// `hermes_constants.get_hermes_home()`: context override -> env var ->
/// platform default.
///
/// Degrades to `.` when nothing resolves - it never fails, because every
/// caller on a live path would otherwise have to invent a policy. A caller
/// that must distinguish the degraded case asks [`Opt::Fn`] instead.
pub fn Fn() -> PathBuf { HermesHomeOpt().unwrap_or_else(|| PathBuf::from(".")) }
