//! `Reload`: rescan the script dirs.

use std::time::SystemTime;

use rhai::Engine as Rhai;

use crate::Scripting::{Engine::Struct::Struct, Script::Struct::Struct as Script};

impl Struct {
	/// Rescan the script dirs, reloading every `.rhai` file that compiles.
	/// Scripts that fail to compile are warned and dropped (degrade, never
	/// panic).
	///
	/// The whole list is cleared first: a partial reload would keep a script
	/// that was deleted, and a deleted script that keeps running is worse than
	/// a broken one that does not. A compile failure is dropped here rather than
	/// kept for a retry - the next mtime change brings it back.
	pub fn Reload(&self) {
		let mut Scripts = self.Scripts.lock().unwrap_or_else(|e| e.into_inner());
		Scripts.clear();

		self.dirs.iter().for_each(|Directory| {
			let Ok(Entries) = std::fs::read_dir(Directory) else { return };

			Entries.flatten().for_each(|Entry| {
				let path = Entry.path();

				match path.extension().map(|e| e == "rhai").unwrap_or(false) {
					true => {
						let Ok(Source) = std::fs::read_to_string(&path) else { return };
						let Mtime = Entry
							.Metadata()
							.ok()
							.and_then(|m| m.modified().ok())
							.unwrap_or(SystemTime::UNIX_EPOCH);
						let Engine = Rhai::new();

						match Engine.compile(&Source) {
							Ok(_) => {
								tracing::info!(Script = %path.display(), "loaded");
								Scripts.push(Script { path, Mtime, Source });
							},
							Err(e) => {
								tracing::warn!(Script = %path.display(), error = %e, "script failed to compile - skipped");
							},
						}
					},
					false => return,
				}
			});
		});
	}
}
