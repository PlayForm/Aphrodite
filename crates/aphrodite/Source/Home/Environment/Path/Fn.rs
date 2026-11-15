//! `EnvironmentPath`: a non-empty env var as a path.

use std::path::PathBuf;

/// Non-empty env var value as a path, if set.
///
/// An empty value is treated as UNSET, matching the Python shim, which strips
/// empty values the same way. Returning `Some("")` here would make every
/// precedence chain below produce an empty path component instead of falling
/// through to the next level - the failure looks like "config ignored", not
/// like "env var empty", which is why the check lives in this one function.
pub(crate) fn Fn(name:&str) -> Option<PathBuf> { std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from) }
