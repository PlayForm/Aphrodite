//! `CheckReload`: reload when a loaded script changed on disk.

use crate::Scripting::Engine::Struct::Struct;

impl Struct {
	/// Hot-reload when any loaded script's mtime changed on disk.
	///
	/// Only ALREADY-LOADED scripts are watched: a newly created file is picked up on the
	/// next process start, not mid-session. Watching the directory would reload on every
	/// unrelated write in it, and a reload compiles every script - which is exactly the
	/// cost this check exists to avoid.
	///
	/// The mtime test lives inside `any`, so a script that has vanished from disk answers
	/// `false` for that entry rather than aborting the scan: a deleted script is a reload
	/// trigger only when its metadata is still readable, and the next `Reload` drops it
	/// either way.
	pub(crate) fn CheckReload(&self) {
		let Needs = self
			.Scripts
			.lock()
			.unwrap_or_else(|Error| Error.into_inner())
			.iter()
			.any(|Script| {
				std::fs::metadata(&Script.Path)
					.ok()
					.and_then(|Metadata| Metadata.modified().ok())
					.map(|Modified| Modified > Script.Mtime)
					.unwrap_or(false)
			});

		// The guard above is released at the end of its statement, which is what lets
		// `Reload` take the same mutex.
		let _ = Needs.then(|| self.Reload());
	}
}
