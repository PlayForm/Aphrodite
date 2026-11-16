//! Recording a tool event into the bounded ring.

use super::{Const::Cap, Entry::Struct::Struct as Entry, Struct::Struct};

impl Struct {
	/// Record a per-call tool event into the bounded ring (P2/T6). Caps at
	/// `Event::Const::Cap` (200) entries, evicting the front (oldest) - same
	/// eviction style as `Marker::Recent`.
	///
	/// The number to drop is arithmetic on a length the ring already knows
	/// (`len - cap`), not a pop loop: one append can overfill the ring by at most
	/// one, and `drain` removes exactly the excess from the front, where the
	/// oldest events sit.
	pub fn Record(&mut self, Event:Entry) {
		self.Recent.push_back(Event);

		let Excess = self.Recent.len().saturating_sub(Cap);

		let _ = self.Recent.drain(..Excess);
	}
}
