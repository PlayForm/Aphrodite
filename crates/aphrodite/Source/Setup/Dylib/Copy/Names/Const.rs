//! The shared libraries this platform installs, by bare filename.

#![allow(non_upper_case_globals)]

use super::super::Requirement;

/// The two dylib filenames and how much each one matters.
///
/// These are the *installed* names - the name the loader opens - not the release
/// asset names, which carry the target triple. One definition exists per target,
/// chosen by `#[cfg]`, so there is no branch at run time and no table a reader has
/// to check against the platform they are on.
#[cfg(target_os = "macos")]
pub(crate) const Const:&[(&str, Requirement::Enum::Enum)] = &[
	("libaphrodite.dylib", Requirement::Enum::Enum::Optional),
	("libaphrodite_hermes.dylib", Requirement::Enum::Enum::Required),
];

#[cfg(target_os = "linux")]
pub(crate) const Const:&[(&str, Requirement::Enum::Enum)] = &[
	("libaphrodite.so", Requirement::Enum::Enum::Optional),
	("libaphrodite_hermes.so", Requirement::Enum::Enum::Required),
];

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) const Const:&[(&str, Requirement::Enum::Enum)] = &[
	("aphrodite.dll", Requirement::Enum::Enum::Optional),
	("aphrodite_hermes.dll", Requirement::Enum::Enum::Required),
];
