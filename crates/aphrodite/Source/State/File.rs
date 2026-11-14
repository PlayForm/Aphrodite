use super::{AphroditeState, REFERENCED_FILES_CAP};

impl AphroditeState {
	/// Record a referenced file.
	pub fn record_file(&mut self, path:String, tool:String) {
		self.referenced_files.retain(|(p, _)| p != &path);
		self.referenced_files.push_front((path, tool));
		while self.referenced_files.len() > REFERENCED_FILES_CAP {
			self.referenced_files.pop_back();
		}
	}
}
