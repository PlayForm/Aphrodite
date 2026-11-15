//! The setup error type.
//!
//! Setup fails in four ways the user has to be able to tell apart: the filesystem
//! refused an operation, `hermes` is absent or not answering, a dylib could not
//! be located or downloaded, or `hermes plugins enable` failed. Each variant
//! carries the failing operation and input in its message, because setup prints
//! the error and exits - there is no later step that can diagnose it.

use std::io;

/// Failure of the setup/bootstrap path.
///
/// `Io` wraps every filesystem and `current_exe` failure through `#[from]`, so a
/// write that cannot happen names the path that could not be written. The other
/// variants carry the failing command's own stderr.
///
/// It is not a retryable error hierarchy: no caller matches on a variant to
/// decide what to do next, and no variant has a source chain. The variants exist
/// for the message and for `Debug` output, so adding one is cheap but changing
/// an existing message is a user-visible change.
#[derive(Debug, thiserror::Error)]
pub enum Enum {
	#[error("I/O error: {0}")]
	Io(#[from] io::Error),
	#[error("{0}")]
	HermesNotFound(String),
	#[error("{0}")]
	DylibNotFound(String),
	#[error("{0}")]
	PluginRegistrationFailed(String),
}
