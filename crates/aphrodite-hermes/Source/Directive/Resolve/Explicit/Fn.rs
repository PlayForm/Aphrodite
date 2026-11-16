//! The explicit caller-supplied home, the first candidate.

use std::path::PathBuf;

/// `<Override>/directives/`, when the caller supplied a home.
///
/// Taken verbatim: this is not a validator, so a home that does not exist yet is
/// still the answer - provisioning reports the create failure as a warning
/// instead of refusing the call.
pub(crate) fn Fn(Override:&str) -> Option<PathBuf> {
	(!Override.is_empty()).then(|| PathBuf::from(Override).join("directives"))
}
