//! The Rust target triple of the running platform.

#![allow(non_upper_case_globals)]

/// The target triple this binary was compiled for.
///
/// Release assets are published per triple, so the download path has to name the
/// platform it is on. This is compile-time selection, not a runtime probe: the
/// binary that runs setup was already built for exactly one platform, and asking
/// the OS could answer with a different one than the artifact was built for. The
/// `#[cfg]` on each definition is what makes that selection - the item itself is
/// chosen by the compiler, so there is no branch left to read.
///
/// Covers the four platforms Aphrodite publishes for - macOS on both
/// architectures, and x86_64 Linux and Windows. Anything else yields `"unknown"`,
/// which produces a release URL that cannot exist.
///
/// It is not exhaustive: a new publish target has to be added here as well as to
/// the release matrix, or setup will look for an asset named
/// `...-unknown.dylib` and fail with a download error rather than a clear
/// "unsupported platform".
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) const Const:&str = "aarch64-apple-darwin";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub(crate) const Const:&str = "x86_64-apple-darwin";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const Const:&str = "x86_64-unknown-linux-gnu";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) const Const:&str = "x86_64-pc-windows-msvc";

#[cfg(not(any(
	all(target_os = "macos", target_arch = "aarch64"),
	all(target_os = "macos", target_arch = "x86_64"),
	all(target_os = "linux", target_arch = "x86_64"),
	all(target_os = "windows", target_arch = "x86_64"),
)))]
pub(crate) const Const:&str = "unknown";
