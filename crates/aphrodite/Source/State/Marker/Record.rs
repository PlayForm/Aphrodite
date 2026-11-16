//! Recording a marker into the catalog.

use super::{Const::Cap, Entry::Struct::Struct as Entry, Struct::Struct};

impl Struct {
	/// Record a compression marker.
	///
	/// The cap is arithmetic on the length the ring already knows (`len - cap`),
	/// drained from the front - so index 0 is always the oldest survivor. The
	/// tool-event ring uses the same expression, which is what lets a reader
	/// assume "oldest at index 0" for both without checking which convention a
	/// ring picked.
	pub fn Record(&mut self, Entry:Entry) {
		// DEV-ONLY debug metrics (crate::Debug): one atomic add when the
		// logger is on; a no-op single load when off.
		crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::MarkersCreated);
		self.Recent.push_back(Entry);

		// Keep the last `Marker::Const::Cap` markers; front eviction (fix 6)
		// keeps index 0 as the oldest survivor.
		let Excess = self.Recent.len().saturating_sub(Cap);

		let _ = self.Recent.drain(..Excess);
	}
}
