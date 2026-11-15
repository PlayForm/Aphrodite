//! Reading `aphrodite.toml` into the typed model.

use super::super::Struct::Struct;

impl Struct {
	/// Load from the given aphrodite.toml path.
	///
	/// Unlike the Hermes-side loader this one is strict: a missing file, an
	/// unreadable file and a file that does not deserialise all return an
	/// error. The engine binary decides what to do about that (fall back to
	/// CLI values, or report), which is the difference between the two
	/// loaders and the reason both exist.
	pub fn Load(path:&str) -> anyhow::Result<Self> {
		let Content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&Content)?)
	}
}
