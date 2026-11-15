//! The installed binary's filename.

#![allow(non_upper_case_globals)]

/// The binary filename for this platform.
///
/// Setup copies itself into the runtime home's `binaries/` directory under this
/// name, and the loader later launches that same path, so one value owns the name
/// and the copy and the launch cannot disagree about it. The `#[cfg]` selects the
/// definition rather than branching at run time: there is exactly one name per
/// target and the compiler picks it.
///
/// It is not a path and it is not configurable: the name is fixed by the release
/// asset and by what the loader looks for, so a value from configuration would
/// produce an install nothing launches. A wrong name here installs a working
/// binary under a name the plugin never opens, and the plugin reports a missing
/// binary rather than a bad install.
#[cfg(target_os = "windows")]
pub(crate) const Const:&str = "aphrodite.exe";

#[cfg(not(target_os = "windows"))]
pub(crate) const Const:&str = "aphrodite";
