//! Reading `aphrodite.toml` into the typed model.

use std::path::Path;

use super::super::Struct::Struct;

impl Struct {
	/// Load from the given aphrodite.toml path.
	///
	/// Unlike the Hermes-side loader this one is strict: a missing file, an unreadable
	/// file and a file that does not deserialise all return an error. The engine binary
	/// decides what to do about that (fall back to CLI values, or report), which is the
	/// difference between the two loaders and the reason both exist.
	///
	/// The path is `impl AsRef<Path>` rather than `&str` because the callers hold two
	/// different spellings of it: the command line's `String` and the watcher's resolved
	/// `PathBuf`. A `&str` signature forced the watcher to lossily convert a path it had
	/// already resolved - and a lossy conversion of a non-UTF-8 path is a *different*
	/// file, so a reload would have read nothing and reported success.
	pub fn Load(Path:impl AsRef<Path>) -> anyhow::Result<Self> {
		let Content = std::fs::read_to_string(Path.as_ref())?;

		Ok(toml::from_str(&Content)?)
	}
}
