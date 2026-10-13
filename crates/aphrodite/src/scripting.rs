//! Rhai scripting engine - user-defined micro-scripts.
//!
//! Port of the pre-Rust `scripting.rs`, feature-gated behind the `scripting`
//! Cargo feature (`pub mod scripting` in lib.rs is only compiled with the
//! feature on, so the default cdylib build never links rhai).
//!
//! Contract: a script is a `.rhai` file declaring a zero-argument hook
//! function (e.g. `fn on_compress() { ... }`). The engine compiles scripts
//! per call (no AST storage - avoids Send issues), seeds a scope with the
//! hook payload, calls the named function, and chains each script's output
//! into the next script's `content`. Any script error is warned and skipped
//! (degrade to the previous value) - a failing script must never crash the
//! proxy.

use std::{path::PathBuf, sync::Mutex, time::SystemTime};

use rhai::{Engine, Scope};

/// Hook payload handed to a script. Each field is exposed to the script as a
/// scope variable of the same name (`content`, `content_type`, `size`,
/// `hash`, `metadata`, `preview`). `content` is the primary transform target:
/// the value a script returns becomes the next script's `content` (and the
/// returned payload of the hook). `preview` mirrors the chained `content` so
/// the pre-Rust `on_marker` scripts (which chain on the preview) behave
/// identically.
#[derive(Debug, Clone, Default)]
pub struct HookPayload {
	pub content: Option<String>,
	pub content_type: Option<String>,
	pub size: Option<usize>,
	pub hash: Option<String>,
	pub metadata: Option<String>,
	pub preview: Option<String>,
}

/// A single loaded script file.
struct Script {
	path: PathBuf,
	mtime: SystemTime,
	source: String,
}

/// Rhai micro-script engine: loads `.rhai` files from the script dirs,
/// evaluates their hook functions against a [`HookPayload`], and degrades
/// (warn + pass-through) on any script error - never panics.
pub struct ScriptEngine {
	scripts: Mutex<Vec<Script>>,
	dirs: Vec<PathBuf>,
}

impl ScriptEngine {
	/// Create the engine and load every `.rhai` file from the default script
	/// dirs (`~/.hermes/aphrodite/scripts` and `scripts/aphrodite` relative to
	/// the cwd). Missing dirs are skipped silently.
	pub fn new() -> Self {
		let home = dirs::home_dir().unwrap_or_default();
		let engine = ScriptEngine {
			scripts: Mutex::new(Vec::new()),
			dirs: vec![
				home.join(".hermes").join("aphrodite").join("scripts"),
				PathBuf::from("scripts/aphrodite"),
			],
		};
		engine.reload();
		engine
	}

	/// Rescan the script dirs, reloading every `.rhai` file that compiles.
	/// Scripts that fail to compile are warned and dropped (degrade, never
	/// panic).
	pub fn reload(&self) {
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
				let engine = Engine::new();
				if let Err(e) = engine.compile(&source) {
					tracing::warn!(script = %path.display(), error = %e, "script failed to compile - skipped");
					continue;
				}
				tracing::info!(script = %path.display(), "loaded");
				scripts.push(Script { path, mtime, source });
			}
		}
	}

	/// Hot-reload when any loaded script's mtime changed on disk.
	fn check_reload(&self) {
		let needs = self
			.scripts
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.iter()
			.any(|s| {
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

	/// Evaluate every loaded script's `fn <hook>()` against `payload`,
	/// chaining each script's output into the next script's `content`.
	///
	/// Returns `Some(transformed)` when at least one script modified the
	/// payload, `None` when nothing changed. A script that errors (throws,
	/// returns the wrong type, panics) is warned and skipped - its input
	/// passes through unchanged; a script that does not define the hook
	/// function is skipped silently (missing function is not an error). All
	/// lock/compile/call sites degrade; this method never panics.
	pub fn run(&self, hook: &str, payload: &HookPayload) -> Option<String> {
		self.check_reload();
		let scripts = self.scripts.lock().unwrap_or_else(|e| e.into_inner());
		let mut current = payload.content.clone().unwrap_or_default();
		let mut changed = false;
		for script in scripts.iter() {
			let engine = Engine::new();
			// Compiled per call - no AST storage, avoids Send issues. Source
			// was compile-validated at reload, so a failure here means the
			// script was injected directly - skip silently.
			let Ok(ast) = engine.compile(&script.source) else { continue };
			// Scripts that don't define the hook function opt out - a
			// zero-parameter `fn <hook>()` reading the seeded scope (missing
			// function is not an error).
			let has_hook = ast.iter_functions().any(|f| f.name == hook && f.params.is_empty());
			if !has_hook {
				continue;
			}
			let mut scope = Scope::new();
			// `push` only fails on duplicate names - the names below are
			// distinct, so the Result is deliberately ignored.
			let _ = scope.push("content", current.clone());
			let _ = scope.push("hook", hook.to_string());
			let _ = scope.push("content_type", payload.content_type.clone().unwrap_or_default());
			let _ = scope.push("size", payload.size.map(|s| s as i64).unwrap_or(0));
			let _ = scope.push("hash", payload.hash.clone().unwrap_or_default());
			let _ = scope.push("metadata", payload.metadata.clone().unwrap_or_default());
			let _ = scope.push("preview", current.clone());
			match engine.call_fn::<String>(&mut scope, &ast, hook, ()) {
				Ok(transformed) => {
					current = transformed;
					changed = true;
				},
				Err(e) => {
					tracing::warn!(
						script = %script.path.display(),
						hook,
						error = %e,
						"script hook failed - degraded to previous value"
					);
				},
			}
		}
		changed.then_some(current)
	}

	/// Pre-Rust compatibility surface: run the `on_compress` hook over
	/// content. Returns the transformed content, or the input unchanged when
	/// no script modified it.
	pub fn on_compress(&self, content: &str, ct: &str, size: usize) -> String {
		let payload = HookPayload {
			content: Some(content.to_string()),
			content_type: Some(ct.to_string()),
			size: Some(size),
			..Default::default()
		};
		self.run("on_compress", &payload).unwrap_or_else(|| content.to_string())
	}

	/// Pre-Rust compatibility surface: run the `on_marker` hook over the
	/// preview. Returns the transformed preview, or the input unchanged when
	/// no script modified it.
	pub fn on_marker(&self, hash: &str, ct: &str, meta: &str, preview: &str) -> String {
		let payload = HookPayload {
			content: Some(preview.to_string()),
			content_type: Some(ct.to_string()),
			hash: Some(hash.to_string()),
			metadata: Some(meta.to_string()),
			preview: Some(preview.to_string()),
			..Default::default()
		};
		self.run("on_marker", &payload).unwrap_or_else(|| preview.to_string())
	}

	/// Pre-Rust compatibility surface: run the `on_retrieve` hook over
	/// content. Returns the transformed content, or the input unchanged when
	/// no script modified it.
	pub fn on_retrieve(&self, hash: &str, content: &str, ct: &str) -> String {
		let payload = HookPayload {
			content: Some(content.to_string()),
			content_type: Some(ct.to_string()),
			hash: Some(hash.to_string()),
			..Default::default()
		};
		self.run("on_retrieve", &payload).unwrap_or_else(|| content.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn script(source: &str) -> Script {
		Script {
			path: PathBuf::from("test.rhai"),
			mtime: SystemTime::UNIX_EPOCH,
			source: source.to_string(),
		}
	}

	fn engine(scripts: Vec<Script>) -> ScriptEngine {
		ScriptEngine { scripts: Mutex::new(scripts), dirs: Vec::new() }
	}

	// ── Contract: a script evaluates against a hook payload and returns the
	// modified payload (Some) ──
	#[test]
	fn script_transforms_payload() {
		let engine = engine(vec![script(
			r#"fn on_compress() {
				content + " [size=" + size.to_string() + " type=" + content_type + "]"
			}"#,
		)]);
		let payload = HookPayload {
			content: Some("hello".to_string()),
			content_type: Some("text".to_string()),
			size: Some(5),
			..Default::default()
		};
		let out = engine.run("on_compress", &payload);
		assert_eq!(out.as_deref(), Some("hello [size=5 type=text]"));
	}

	// ── Contract: scripts chain - each script's output feeds the next
	// script's `content` ──
	#[test]
	fn script_output_chains_into_next_script() {
		let engine = engine(vec![
			script("fn on_compress() { content + \"A\" }"),
			script("fn on_compress() { content + \"B\" }"),
		]);
		let payload = HookPayload { content: Some("x".to_string()), ..Default::default() };
		assert_eq!(engine.run("on_compress", &payload).as_deref(), Some("xAB"));
	}

	// ── Contract: a script that errors (throws, returns the wrong type)
	// degrades with a warn - no panic - and the other scripts still apply ──
	#[test]
	fn script_error_degrades_without_panic() {
		let engine = engine(vec![
			script(r#"fn on_compress() { throw "boom" }"#),
			script("fn on_compress() { 42 }"),
			script("fn on_compress() { content + \"!\" }"),
		]);
		let payload = HookPayload { content: Some("x".to_string()), ..Default::default() };
		let out = engine.run("on_compress", &payload);
		assert_eq!(out.as_deref(), Some("x!"), "bad scripts skipped, good script applied");
	}

	// ── Contract: a script that does not define the hook function opts out
	// silently - no warn, no change ──
	#[test]
	fn missing_hook_fn_is_skipped_silently() {
		let engine = engine(vec![script("fn unrelated() { 1 }")]);
		let payload = HookPayload { content: Some("x".to_string()), ..Default::default() };
		assert_eq!(engine.run("on_compress", &payload), None);
	}

	// ── Contract: no scripts = pass-through unchanged (legacy surface) ──
	#[test]
	fn no_scripts_passes_through_unchanged() {
		let engine = engine(Vec::new());
		assert_eq!(engine.on_compress("abc", "text", 3), "abc");
		assert_eq!(engine.on_marker("hash", "text", "meta", "prev"), "prev");
		assert_eq!(engine.on_retrieve("hash", "abc", "text"), "abc");
	}

	// ── Contract: legacy on_marker chains on the preview ──
	#[test]
	fn on_marker_transforms_preview() {
		let engine = engine(vec![script("fn on_marker() { preview + \"|\" + hash + \"|\" + metadata }")]);
		assert_eq!(engine.on_marker("h1", "text", "m1", "prev"), "prev|h1|m1");
	}

	// ── Contract: a script that fails to compile is warned and dropped at
	// reload - the engine degrades to no-op rather than crashing ──
	#[test]
	fn broken_script_is_dropped_at_reload() {
		let dir = std::env::temp_dir().join(format!(
			"aphrodite-scripting-reload-{}",
			std::process::id()
		));
		let _ = std::fs::create_dir_all(&dir);
		std::fs::write(dir.join("broken.rhai"), "fn on_compress() { this is not valid rhai !!").unwrap();
		let engine = ScriptEngine { scripts: Mutex::new(Vec::new()), dirs: vec![dir.clone()] };
		engine.reload();
		let _ = std::fs::remove_dir_all(&dir);
		assert!(engine.scripts.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
		let payload = HookPayload { content: Some("x".to_string()), ..Default::default() };
		assert_eq!(engine.run("on_compress", &payload), None);
	}

	// ── Gate documentation: this module only exists when the feature is on
	// (lib.rs declares `pub mod scripting` behind `#[cfg(feature = "scripting")]`) ──
	#[test]
	fn scripting_module_requires_feature() {
		assert!(cfg!(feature = "scripting"));
	}
}