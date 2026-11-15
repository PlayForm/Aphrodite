//! Recording a referenced file.

use crate::State::{Const::ReferencedFilesCap, Struct::AphroditeState};

impl AphroditeState {
	/// Record a referenced file.
	pub fn RecordFile(&mut self, path:String, Tool:String) {
		self.ReferencedFiles.retain(|(p, _)| p != &path);
		self.ReferencedFiles.push_front((path, Tool));

		loop {
			match self.ReferencedFiles.len() > ReferencedFilesCap {
				true => {
					self.ReferencedFiles.pop_back();
				},
				false => break,
			}
		}
	}
}
