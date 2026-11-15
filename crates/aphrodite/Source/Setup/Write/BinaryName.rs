//! The installed binary's filename.
//!
//! Setup copies itself into the runtime home's `binaries/` directory under this
//! name, and the loader later launches that same path. One function owns the name
//! so the copy and the launch cannot disagree about it.

/// The binary filename for this platform: `aphrodite.exe` on Windows, `Aphrodite`
/// everywhere else.
///
/// It is not a path and it is not configurable: the name is fixed by the release
/// asset and by what the loader looks for, so a value from configuration would
/// produce an install nothing launches. A wrong name here installs a working binary
/// under a name the plugin never opens, and the plugin reports a missing binary
/// rather than a bad install.
pub(crate) fn Fn() -> &'static str { if cfg!(target_os = "windows") { "aphrodite.exe" } else { "aphrodite" } }
