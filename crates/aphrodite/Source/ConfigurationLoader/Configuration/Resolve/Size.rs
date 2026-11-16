//! Size knobs - the `usize` view of the integer chain.

use super::super::Struct::Struct;

impl Struct {
	/// Resolve usize: same as u64 but for sizes.
	///
	/// Resolution is delegated verbatim, so the precedence and the
	/// fall-back-on-unparseable behavior are the integer chain's, not a
	/// second copy of it.
	pub fn Size(&self, EnvironmentKey:&str, Section:&str, Key:&str, Fallback:usize) -> usize {
		self.Number(EnvironmentKey, Section, Key, Fallback as u64) as usize
	}
}
