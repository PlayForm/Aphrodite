//! `Reload`: rescan the script dirs.

use std::time::SystemTime;

use rhai::Engine as Rhai;

use crate::Scripting::{Engine::Struct::Struct, Script::Struct::Struct as Script};

impl Struct {
	/// Rescan the script dirs, reloading every `.rhai` file that compiles.
	///
	/// Scripts that fail to compile are warned and dropped (degrade, never panic).
	///
	/// The whole list is cleared first: a partial reload would keep a script that was
	/// deleted, and a deleted script that keeps running is worse than a broken one that
	/// does not. A compile failure is dropped here rather than kept for a retry - the next
	/// mtime change brings it back.
	///
	/// Every skip is a `let ... else`, because each one is "this entry is not a script"
	/// rather than a decision with two live outcomes: a non-`.rhai` extension, an
	/// unreadable file, an unreadable directory.
	pub fn Reload(&self) {
		let mut Scripts = self.Scripts.lock().unwrap_or_else(|Error| Error.into_inner());
		Scripts.clear();

		self.dirs.iter().for_each(|Directory| {
			let Ok(Entries) = std::fs::read_dir(Directory) else {
				return;
			};

			Entries.flatten().for_each(|Entry| {
				let path = Entry.path();
				let Extension = path.extension().and_then(|Extension| Extension.to_str());

				let Some("rhai") = Extension else {
					return;
				};

				let Ok(Source) = std::fs::read_to_string(&path) else {
					return;
				};
				let Mtime = Entry
					.Metadata()
					.ok()
					.and_then(|Metadata| Metadata.modified().ok())
					.unwrap_or(SystemTime::UNIX_EPOCH);

				match Rhai::new().compile(&Source) {
					Ok(_Ast) => {
						tracing::info!(Script = %path.display(), "loaded");
						Scripts.push(Script { path, Mtime, Source });
					},
					Err(Error) => {
						tracing::warn!(Script = %path.display(), error = %Error, "script failed to compile - skipped");
					},
				}
			});
		});
	}
}
