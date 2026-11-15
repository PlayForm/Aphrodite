//! A verified artifact that has taken its platform's preparation.

use std::path::PathBuf;

/// A verified artifact with its final permissions already applied.
///
/// Preparation happens **before** publication so that nothing is ever readable at
/// the destination with the wrong mode. The window is small either way, but a file
/// that lands world-readable and is chmodded a moment later has still been
/// world-readable for that moment.
pub(crate) struct Struct {
	/// The staged path, now with its final mode.
	pub Path:PathBuf,
	/// Where it is going.
	pub Dest:PathBuf,
}
