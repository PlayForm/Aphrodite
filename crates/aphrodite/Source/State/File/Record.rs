//! Recording a referenced file.

use super::super::{AphroditeState, Const::ReferencedFilesCap};

impl AphroditeState {
	/// Record a referenced file.
	pub fn RecordFile(&mut self, path:String, Tool:String) {
		self.ReferencedFiles.retain(|(p, _)| p != &path);
		self.ReferencedFiles.push_front((path, Tool));
		while self.ReferencedFiles.len() > ReferencedFilesCap {
			self.ReferencedFiles.pop_back();
		}
	}
}
