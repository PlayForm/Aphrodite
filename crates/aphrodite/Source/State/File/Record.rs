//! Recording a referenced file.

use super::{Const::Cap, Struct::Struct};

impl Struct {
	/// Record a referenced file.
	///
	/// The newest entry goes to the front and the list is then truncated to the
	/// cap, which drops from the back - the least-recently-referenced end. That
	/// is one call over a known length instead of a pop loop, and it preserves
	/// the ordering the ring is for: front is most recent.
	pub fn Record(&mut self, path:String, Tool:String) {
		self.ReferencedFiles.retain(|(p, _)| p != &path);
		self.ReferencedFiles.push_front((path, Tool));
		self.ReferencedFiles.truncate(Cap);
	}
}
