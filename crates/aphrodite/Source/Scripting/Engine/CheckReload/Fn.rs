//! `CheckReload`: reload when a loaded script changed on disk.

use crate::Scripting::Engine::Struct::Struct;

impl Struct {
	/// Hot-reload when any loaded script's mtime changed on disk.
	///
	/// Only ALREADY-LOADED scripts are watched: a newly created file is picked
	/// up on the next process start, not mid-session. Watching the directory
	/// would reload on every unrelated write in it, and a reload compiles every
	/// script - which is exactly the cost this check exists to avoid.
	pub(crate) fn CheckReload(&self) {
		let Needs = self.Scripts.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|s| {
			std::fs::metadata(&s.path)
				.ok()
				.and_then(|m| m.modified().ok())
				.map(|t| t > s.Mtime)
				.unwrap_or(false)
		});
		if Needs {
			self.Reload();
		}
	}
}
