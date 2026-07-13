//! Rhai scripting engine - user-defined micro-scripts.
//! Feature-gated behind `APHRODITE_SCRIPTING=1`.
//! Compiles scripts per-call (no AST storage - avoids Send issues).

#[cfg(feature = "scripting")]
mod engine {
	use std::{path::PathBuf, sync::Mutex, time::SystemTime};

	use rhai::{Engine, Scope};

	struct Script {
		path:PathBuf,
		mtime:SystemTime,
		source:String,
	}

	pub struct ScriptEngine {
		scripts:Mutex<Vec<Script>>,
		dirs:Vec<PathBuf>,
	}

	impl ScriptEngine {
		/// Create a new engine and load scripts from the user
		/// (`~/.hermes/aphrodite/scripts`) and project-local (`scripts/aphrodite`)
		/// directories.
		pub fn new() -> Self {
			let home = dirs::home_dir().unwrap_or_default();
			let se = ScriptEngine {
				scripts:Mutex::new(Vec::new()),
				dirs:vec![
					home.join(".hermes").join("aphrodite").join("scripts"),
					PathBuf::from("scripts/aphrodite"),
				],
			};
			se.reload();
			se
		}

		fn reload(&self) {
			let mut scripts = self.scripts.lock().unwrap_or_else(|e| e.into_inner());
			scripts.clear();
			for dir in &self.dirs {
				let Ok(entries) = std::fs::read_dir(dir) else { continue };
				for entry in entries.flatten() {
					let path = entry.path();
					if !path.extension().map(|e| e == "rhai").unwrap_or(false) {
						continue;
					}
					let Ok(source) = std::fs::read_to_string(&path) else { continue };
					let mtime = entry
						.metadata()
						.ok()
						.and_then(|m| m.modified().ok())
						.unwrap_or(SystemTime::UNIX_EPOCH);
					// Validate compilation
					let engine = Engine::new();
					if engine.compile(&source).is_ok() {
						tracing::info!(script=%path.display(), "loaded");
						scripts.push(Script { path, mtime, source });
					}
				}
			}
		}

		fn check_reload(&self) {
			let needs = self.scripts.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|s| {
				std::fs::metadata(&s.path)
					.ok()
					.and_then(|m| m.modified().ok())
					.map(|t| t > s.mtime)
					.unwrap_or(false)
			});
			if needs {
				self.reload();
			}
		}

		/// Run all loaded scripts' `on_compress` hook in sequence, threading the
		/// result of each through to the next. Scripts that fail to expose the
		/// hook (or error) are skipped, leaving `result` unchanged.
		pub fn on_compress(&self, content:&str, ct:&str, size:usize) -> String {
			self.check_reload();
			let scripts = self.scripts.lock().unwrap_or_else(|e| e.into_inner());
			let mut result = content.to_string();
			for s in scripts.iter() {
				let engine = Engine::new();
				let Ok(ast) = engine.compile(&s.source) else { continue };
				let mut scope = Scope::new();
				scope.push("content", result.clone());
				scope.push("content_type", ct.to_string());
				scope.push("size", size as i64);
				if let Ok(r) = engine.call_fn::<String>(&mut scope, &ast, "on_compress", ()) {
					result = r;
				}
			}
			result
		}

		/// Run all loaded scripts' `on_marker` hook in sequence, letting each
		/// script rewrite the CCR marker preview text before it's shown to the
		/// agent.
		pub fn on_marker(&self, hash:&str, ct:&str, meta:&str, preview:&str) -> String {
			self.check_reload();
			let scripts = self.scripts.lock().unwrap_or_else(|e| e.into_inner());
			let mut result = preview.to_string();
			for s in scripts.iter() {
				let engine = Engine::new();
				let Ok(ast) = engine.compile(&s.source) else { continue };
				let mut scope = Scope::new();
				scope.push("hash", hash.to_string());
				scope.push("content_type", ct.to_string());
				scope.push("metadata", meta.to_string());
				scope.push("preview", result.clone());
				if let Ok(r) = engine.call_fn::<String>(&mut scope, &ast, "on_marker", ()) {
					result = r;
				}
			}
			result
		}

		/// Run all loaded scripts' `on_retrieve` hook in sequence, letting each
		/// script post-process retrieved content before it's returned to the
		/// agent.
		pub fn on_retrieve(&self, hash:&str, content:&str, ct:&str) -> String {
			self.check_reload();
			let scripts = self.scripts.lock().unwrap_or_else(|e| e.into_inner());
			let mut result = content.to_string();
			for s in scripts.iter() {
				let engine = Engine::new();
				let Ok(ast) = engine.compile(&s.source) else { continue };
				let mut scope = Scope::new();
				scope.push("hash", hash.to_string());
				scope.push("content", result.clone());
				scope.push("content_type", ct.to_string());
				if let Ok(r) = engine.call_fn::<String>(&mut scope, &ast, "on_retrieve", ()) {
					result = r;
				}
			}
			result
		}
	}
}

#[cfg(not(feature = "scripting"))]
mod engine {
	pub struct ScriptEngine;
	impl Default for ScriptEngine {
		fn default() -> Self { Self::new() }
	}

	impl ScriptEngine {
		pub fn new() -> Self { ScriptEngine }
		pub fn on_compress(&self, c:&str, _:&str, _:usize) -> String { c.to_string() }
		pub fn on_marker(&self, _:&str, _:&str, _:&str, p:&str) -> String { p.to_string() }
		pub fn on_retrieve(&self, _:&str, c:&str, _:&str) -> String { c.to_string() }
	}
}

pub use engine::ScriptEngine;
