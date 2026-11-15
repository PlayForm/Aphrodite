//! The Rust target triple of the running platform.
//!
//! Release assets are published per triple, so the download path has to name the
//! platform it is on. This is a compile-time table, not a runtime probe: the
//! binary that runs setup is already built for exactly one platform, and asking
//! the OS could answer with a different one than the artifact was built for.

/// The target triple this binary was compiled for.
///
/// Covers the four platforms Aphrodite publishes for - macOS on both
/// architectures, and x86_64 Linux and Windows. Anything else yields `"unknown"`,
/// which produces a release URL that cannot exist.
///
/// It is not a `rustc -vV` read and it is not exhaustive: a new publish target
/// has to be added here as well as to the release matrix, or setup will look for
/// an asset named `...-unknown.dylib` and fail with a download error rather than
/// a clear "unsupported platform".
pub(super) fn Fn() -> &'static str {
	if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
		"aarch64-apple-darwin"
	} else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
		"x86_64-apple-darwin"
	} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
		"x86_64-unknown-linux-gnu"
	} else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
		"x86_64-pc-windows-msvc"
	} else {
		"unknown"
	}
}
