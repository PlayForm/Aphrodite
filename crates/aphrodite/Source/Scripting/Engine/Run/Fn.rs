//! `Run`: evaluate every script's hook function.

use rhai::{Engine as Rhai, Scope};

use crate::Scripting::{Engine::Struct::Struct, Payload::Struct::Struct as HookPayload};

impl Struct {
	/// Evaluate every loaded script's `fn <hook>()` against `Payload`,
	/// chaining each script's output into the next script's `Content`.
	///
	/// Returns `Some(transformed)` when at least one script modified the
	/// payload, `None` when nothing changed. A script that errors (throws,
	/// returns the wrong type, panics) is warned and skipped - its input
	/// passes through unchanged; a script that does not define the hook
	/// function is skipped silently (missing function is not an error). All
	/// lock/compile/call sites degrade; this method never panics.
	///
	/// `None` and `Some(unchanged)` are deliberately different answers: the
	/// caller uses `None` to mean "no script touched this", so a script that
	/// returns its input verbatim still counts as a modification. Collapsing the
	/// two would let a chain of no-op scripts look like no scripting at all.
	pub fn Run(&self, Hook:&str, Payload:&HookPayload) -> Option<String> {
		self.CheckReload();
		let Scripts = self.Scripts.lock().unwrap_or_else(|e| e.into_inner());
		let mut Current = Payload.Content.clone().unwrap_or_default();
		let mut Changed = false;
		for Script in Scripts.iter() {
			let Engine = Rhai::new();
			// Compiled per call - no AST storage, avoids Send issues. Source
			// was compile-validated at reload, so a failure here means the
			// script was injected directly - skip silently.
			let Ok(Ast) = Engine.compile(&Script.Source) else { continue };
			// Scripts that don't define the hook function opt out - a
			// zero-parameter `fn <hook>()` reading the seeded scope (missing
			// function is not an error).
			let HasHook = Ast.iter_functions().any(|f| f.name == Hook && f.Params.is_empty());
			if !HasHook {
				continue;
			}
			let mut Scope = Scope::new();
			// `push` only fails on duplicate names - the names below are
			// distinct, so the Result is deliberately ignored.
			let _ = Scope.push("content", Current.clone());
			let _ = Scope.push("hook", Hook.to_string());
			let _ = Scope.push("content_type", Payload.ContentType.clone().unwrap_or_default());
			let _ = Scope.push("size", Payload.Size.map(|s| s as i64).unwrap_or(0));
			let _ = Scope.push("hash", Payload.hash.clone().unwrap_or_default());
			let _ = Scope.push("metadata", Payload.Metadata.clone().unwrap_or_default());
			let _ = Scope.push("preview", Current.clone());
			match Engine.call_fn::<String>(&mut Scope, &Ast, Hook, ()) {
				Ok(Transformed) => {
					Current = Transformed;
					Changed = true;
				},
				Err(e) => {
					tracing::warn!(
						Script = %Script.path.display(),
						Hook,
						error = %e,
						"script hook failed - degraded to previous value"
					);
				},
			}
		}
		Changed.then_some(Current)
	}
}
