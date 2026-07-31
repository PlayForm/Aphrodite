//! Conversational Directives - lightweight behavioral context for the LLM.
//!
//! Directives are short `.md` files that inject behavioral instructions into
//! the LLM's context via `pre_llm_call`. Unlike file content (which gets
//! compressed into CCR markers), directives are **always inline** - they're
//! compact enough to never need compression, and the engine injects them
//! directly into the LLM's context without any file-system round-trip or
//! retrieval step.
//!
//! Per profile, per user: `directives/*.md` files are swappable at runtime
//! via `aphrodite_directive("swap", "name")`.

use std::{collections::HashMap, path::PathBuf};

/// Built-in directives baked into the binary via `include_str!`.
/// These ship with every installation and are used as fallbacks when no
/// `directives/` directory exists on disk — so a fresh install gets
/// `focus`, `foresight`, `ccr-handling`, `cleanup`, and `explore` without
/// any filesystem setup.
///
/// The on-disk `directives/*.md` files (if any) take precedence: if the
/// directory exists, its `.md` files replace these defaults entirely.
/// Users can also `aphrodite_directive("add", "ccr-handling")` to
/// activate the shipped defaults discovered from the embedded set.
fn builtin_directives() -> Vec<(&'static str, &'static str)> {
	vec![
		("focus", include_str!("builtin_directives/focus.md")),
		("foresight", include_str!("builtin_directives/foresight.md")),
		("ccr-handling", include_str!("builtin_directives/ccr-handling.md")),
		("cleanup", include_str!("builtin_directives/cleanup.md")),
		("explore", include_str!("builtin_directives/explore.md")),
	]
}

/// Load built-in directives from the binary (via `include_str!`), applying the
/// same `MAX_DIRECTIVE_CHARS` cap as `load_directives` does for disk-loaded
/// directives. Returns a `HashMap` ready for `state.directives`.
pub fn loaded_builtins() -> HashMap<String, Directive> {
	let mut directives = HashMap::new();
	for (name, content) in builtin_directives() {
		let content = if content.len() > MAX_DIRECTIVE_CHARS {
			let trunc:String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
			format!("{}…", trunc)
		} else {
			content.to_string()
		};
		directives.insert(name.to_string(), Directive { name:name.to_string(), content });
	}
	directives
}

/// A loaded directive - name and content.
#[derive(Debug, Clone)]
pub struct Directive {
	pub name:String,
	pub content:String,
}

/// Per-file cap applied when a directive `.md` is loaded from disk.
pub const MAX_DIRECTIVE_CHARS:usize = 2000;

/// Cap on the combined injected text across all active directives (01-F5) -
/// `MAX_DIRECTIVE_CHARS` alone doesn't bound this: with several directives
/// active at once, each already-capped body still stacks up in
/// `build_directive_context`'s output.
pub const MAX_COMBINED_CHARS:usize = 4000;

/// Load all `.md` files from a `directives/` directory.
/// Returns a map of name → Directive. Files without `.md` extension are
/// silently skipped.
///
/// If the directory doesn't exist or contains no `.md` files, the built-in
/// directives (baked into the binary via `include_str!`) are used as
/// fallbacks, so a fresh install without a `directives/` directory still
/// gets `focus`, `foresight`, `ccr-handling`, `cleanup`, and `explore`.
pub fn load_directives(dir:&PathBuf) -> HashMap<String, Directive> {
	let mut directives = HashMap::new();
	let mut loaded_from_disk = false;
	if let Ok(entries) = std::fs::read_dir(dir) {
		for entry in entries.flatten() {
			let path = entry.path();
			if path.extension().map(|e| e != "md").unwrap_or(true) {
				continue;
			}
			let Some(name) = path.file_stem().and_then(|n| n.to_str()) else {
				continue;
			};
			let Ok(content) = std::fs::read_to_string(&path) else {
				continue;
			};
			// Trim each directive to a reasonable size.
			let content = if content.len() > MAX_DIRECTIVE_CHARS {
				let trunc:String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
				format!("{}…", trunc)
			} else {
				content
			};
			directives.insert(name.to_string(), Directive { name:name.to_string(), content });
			loaded_from_disk = true;
		}
	}
	// Fallback: if no `directives/` dir on disk (or it's empty), use the
	// built-in directives baked into the binary so a fresh install gets
	// shipped defaults without any filesystem setup.
	if !loaded_from_disk {
		for (name, content) in builtin_directives() {
			let content = if content.len() > MAX_DIRECTIVE_CHARS {
				let trunc:String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
				format!("{}…", trunc)
			} else {
				content.to_string()
			};
			directives.insert(name.to_string(), Directive { name:name.to_string(), content });
		}
	}
	directives
}

/// Build the context string injected into `pre_llm_call`.
/// Format:
/// ```text
/// [directives: focus]
/// focus:
///   focus - stay targeted, minimal tool usage
///   Each turn: use at most 1-2 tools. Prefer retrieval over re-reading.
///   - One primary action per turn
///   - ...
/// ```
/// Returns empty string if no directives are active.
///
/// 01-F5: previously injected only each directive's first line (a markdown
/// title, e.g. `# focus - stay targeted, minimal tool usage`) - the bullets
/// with the actual behavioral instructions never traveled. Now injects the
/// full (per-file `MAX_DIRECTIVE_CHARS`-capped) body, stripped of leading `#`
/// markers, under a combined-output cap so several active directives can't
/// blow past the context budget this feature is supposed to respect.
pub fn build_directive_context(all:&HashMap<String, Directive>, active:&[String]) -> String {
	if active.is_empty() {
		return String::new();
	}
	let names:Vec<&str> = active.iter().map(|s| s.as_str()).collect();
	let mut out = format!("[directives: {}]\n", names.join(", "));
	for name in active {
		if let Some(d) = all.get(name) {
			out.push_str(&format!("{}:\n", d.name));
			for line in d.content.lines() {
				let line = line.trim_start_matches('#').trim();
				if !line.is_empty() {
					out.push_str("  ");
					out.push_str(line);
					out.push('\n');
				}
			}
		}
	}
	if out.len() > MAX_COMBINED_CHARS {
		let trunc:String = out.chars().take(MAX_COMBINED_CHARS).collect();
		out = format!("{}…\n", trunc);
	}
	out
}

/// Handle a directive action (`list`/`swap`/`add`/`remove`/`reset`) against
/// live state. Both `aphrodite_directive` (extern fn) and `aphrodite_dispatch`'s
/// `"directive"` arm delegate here (01-F8) - previously ~40 lines of this
/// logic were duplicated between the two, with divergent error shapes (the
/// extern fn returned an error via a separate top-level `to_json_error` call,
/// the dispatch arm embedded `{"error": ...}` inside an otherwise-success
/// value). This always returns the latter shape - callers pass the result
/// straight through their own success serializer.
pub fn handle_action(state:&mut crate::state::AphroditeState, action:&str, name:&str) -> serde_json::Value {
	match action {
		"list" => {
			// P3/T10: surface ephemeral (nudge/TTL) entries with their expiry so
			// the mechanism is observable.
			let ephemeral:Vec<serde_json::Value> = state
				.ephemeral_directives
				.iter()
				.map(|e| {
					serde_json::json!({
						"name": e.name,
						"inline": e.inline,
						"expires_after_turn": e.expires_after_turn,
					})
				})
				.collect();
			serde_json::json!({
				"available": state.directives.keys().collect::<Vec<&String>>(),
				"active": &state.active_directives,
				"ephemeral": ephemeral,
			})
		},
		"swap" => {
			if state.directives.contains_key(name) {
				state.active_directives = vec![name.to_string()];
				state.manual_directive_turn = Some(state.turn_counter);
				serde_json::json!({"swapped": name, "active": &state.active_directives})
			} else {
				serde_json::json!({"error": format!("unknown directive: {}", name)})
			}
		},
		"add" => {
			if state.directives.contains_key(name) && !state.active_directives.contains(&name.to_string()) {
				state.active_directives.push(name.to_string());
			}
			state.manual_directive_turn = Some(state.turn_counter);
			serde_json::json!({"active": &state.active_directives})
		},
		"remove" => {
			state.active_directives.retain(|d| d != name);
			state.manual_directive_turn = Some(state.turn_counter);
			serde_json::json!({"active": &state.active_directives})
		},
		"reset" => {
			// `reset` clears named actives AND ephemeral nudges AND the manual
			// latch (P3/P6: explicit return to auto mode - see T10/T20).
			state.active_directives.clear();
			state.ephemeral_directives.clear();
			state.manual_directive_turn = None;
			serde_json::json!({"active": &state.active_directives})
		},
		_ => serde_json::json!({"error": format!("unknown action: {} (use list|swap|add|remove|reset)", action)}),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_build_empty() {
		let all = HashMap::new();
		let context = build_directive_context(&all, &[]);
		assert!(context.is_empty());
	}

	#[test]
	fn test_build_with_active() {
		let mut all = HashMap::new();
		all.insert(
			"focus".into(),
			Directive { name:"focus".into(), content:"stay concise\nuse 1-2 tools".into() },
		);
		let context = build_directive_context(&all, &["focus".into()]);
		assert!(context.contains("[directives: focus]"));
		assert!(context.contains("focus:\n"));
		assert!(context.contains("stay concise"));
		assert!(context.contains("use 1-2 tools"));
	}

	// ── 01-F5: real directive files use a leading `#` on multiple lines
	// (not just the title) - injection must strip it from every line and
	// carry the bullet body through, not just the first line. ──
	#[test]
	fn test_build_with_active_injects_full_body_not_just_first_line() {
		let mut all = HashMap::new();
		all.insert(
			"focus".into(),
			Directive {
				name:"focus".into(),
				content:"# focus - stay targeted, minimal tool usage\n\n# Each turn: use at most 1-2 tools.\n\n- One \
				         primary action per turn\n- Prefer aphrodite_retrieve over re-reading"
					.into(),
			},
		);
		let context = build_directive_context(&all, &["focus".into()]);
		assert!(
			context.contains("Each turn: use at most 1-2 tools."),
			"body line missing: {context}"
		);
		assert!(
			context.contains("One primary action per turn"),
			"bullet line missing: {context}"
		);
		assert!(!context.contains('#'), "leading # markers must be stripped: {context}");
	}

	// ── 01-F8: `handle_action` is the single implementation both
	// `aphrodite_directive` and `aphrodite_dispatch`'s `"directive"` arm now
	// delegate to - cover all five actions plus an unknown one directly. ──
	#[test]
	fn test_handle_action_all_actions_and_unknown() {
		let mut state = crate::state::AphroditeState::default();
		state
			.directives
			.insert("focus".into(), Directive { name:"focus".into(), content:"stay focused".into() });

		let r = handle_action(&mut state, "list", "");
		assert_eq!(r["available"], serde_json::json!(["focus"]));
		assert_eq!(r["active"], serde_json::json!([]));

		let r = handle_action(&mut state, "swap", "focus");
		assert_eq!(r["swapped"], "focus");
		assert_eq!(state.active_directives, vec!["focus".to_string()]);

		let r = handle_action(&mut state, "swap", "nonexistent");
		assert!(r["error"].as_str().unwrap().contains("unknown directive"));

		let r = handle_action(&mut state, "remove", "focus");
		assert_eq!(r["active"], serde_json::json!([]));

		let r = handle_action(&mut state, "add", "focus");
		assert_eq!(r["active"], serde_json::json!(["focus"]));

		let r = handle_action(&mut state, "reset", "");
		assert_eq!(r["active"], serde_json::json!([]));
		assert!(state.active_directives.is_empty());

		let r = handle_action(&mut state, "bogus", "");
		assert!(r["error"].as_str().unwrap().contains("unknown action"));
	}

	// ── P3/T9: a one-shot nudge (ttl=1) pushed during turn N renders in turn
	// N+1's context exactly once, then is purged by post_llm_call so it never
	// shows again. ──
	#[test]
	fn test_one_shot_nudge_renders_once_then_purged() {
		let mut s = crate::state::AphroditeState::default();
		s.turn_counter = 5;
		crate::flow::push_nudge(&mut s, "step back and restate the goal", 1);

		// Simulate the turn advancing (post_llm_call runs next_turn then purge):
		// during turn 5 the nudge is live; render at turn 6 must include it.
		s.turn_counter = 6;
		let ctx6 = crate::flow::build_turn_context(&mut s, None);
		assert!(ctx6.contains("[nudge:"), "one-shot nudge must render at turn 6: {ctx6}");
		assert!(ctx6.contains("step back and restate the goal"));

		// post_llm_call advances to turn 7 and purges expired nudges.
		crate::hooks::post_llm_call(&mut s);
		assert_eq!(s.turn_counter, 7);
		let ctx7 = crate::flow::build_turn_context(&mut s, None);
		assert!(!ctx7.contains("[nudge:"), "expired nudge must be gone at turn 7: {ctx7}");
	}

	// ── P3/T10: at most 4 ephemeral entries are stored; a 5th drops the
	// oldest. ──
	#[test]
	fn test_nudge_cap_drops_oldest() {
		let mut s = crate::state::AphroditeState::default();
		for i in 0..5 {
			crate::flow::push_nudge(&mut s, &format!("nudge number {i}"), 10);
		}
		assert_eq!(s.ephemeral_directives.len(), 4, "cap must hold at 4 stored entries");
		assert!(
			!s.ephemeral_directives
				.iter()
				.any(|e| e.inline.as_deref() == Some("nudge number 0")),
			"the oldest nudge must have been dropped"
		);
		assert!(
			s.ephemeral_directives
				.iter()
				.any(|e| e.inline.as_deref() == Some("nudge number 4")),
			"the newest nudge must survive"
		);
	}

	// ── P3/T10: `list` surfaces ephemeral entries; `reset` clears them and the
	// manual latch. ──
	#[test]
	fn test_list_shows_ephemeral_and_reset_clears_them() {
		let mut s = crate::state::AphroditeState::default();
		s.turn_counter = 3;
		crate::flow::push_nudge(&mut s, "watch out", 2);
		s.manual_directive_turn = Some(3);

		let listed = handle_action(&mut s, "list", "");
		let eph = listed["ephemeral"].as_array().expect("ephemeral array");
		assert_eq!(eph.len(), 1);
		assert_eq!(eph[0]["inline"], "watch out");
		assert_eq!(eph[0]["expires_after_turn"], 5);

		handle_action(&mut s, "reset", "");
		assert!(s.ephemeral_directives.is_empty(), "reset must clear ephemeral nudges");
		assert!(s.manual_directive_turn.is_none(), "reset must clear the manual latch");
	}

	// ── P1/T3: any manual mutation latches `manual_directive_turn` to the
	// current turn (P6 override latch). ──
	#[test]
	fn test_manual_mutation_sets_manual_directive_turn() {
		let mut s = crate::state::AphroditeState::default();
		s.turn_counter = 12;
		s.directives
			.insert("focus".into(), Directive { name:"focus".into(), content:"stay focused".into() });
		handle_action(&mut s, "swap", "focus");
		assert_eq!(s.manual_directive_turn, Some(12), "a manual swap must latch the turn");

		// A failed swap (unknown directive) must NOT latch.
		s.manual_directive_turn = None;
		handle_action(&mut s, "swap", "nope");
		assert!(s.manual_directive_turn.is_none(), "a failed swap must not latch");
	}

	#[test]
	fn test_build_with_active_caps_combined_output() {
		let mut all = HashMap::new();
		all.insert(
			"big".into(),
			Directive { name:"big".into(), content:"x".repeat(MAX_DIRECTIVE_CHARS) },
		);
		all.insert(
			"also-big".into(),
			Directive { name:"also-big".into(), content:"y".repeat(MAX_DIRECTIVE_CHARS) },
		);
		let context = build_directive_context(&all, &["big".into(), "also-big".into()]);
		assert!(
			context.len() <= MAX_COMBINED_CHARS + 10,
			"combined output must respect the cap: {} chars",
			context.len()
		);
	}

	// ── 04-T7: load_directives is not cwd-relative (the caller passes an
	// explicit path), so these are hermetic tempdir tests - no cwd mutation
	// risk, unlike apply_compression's own directory search. ──

	/// A unique scratch directory per test, auto-removed on drop.
	struct TempDir(std::path::PathBuf);
	impl TempDir {
		fn new(tag:&str) -> Self {
			let path = std::env::temp_dir().join(format!(
				"aphrodite-directives-test-{tag}-{}",
				std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)
					.unwrap()
					.as_nanos()
			));
			std::fs::create_dir_all(&path).unwrap();
			Self(path)
		}

		fn path(&self) -> std::path::PathBuf { self.0.clone() }
	}
	impl Drop for TempDir {
		fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
	}

	#[test]
	fn test_load_directives_reads_md_files_from_dir() {
		let dir = TempDir::new("basic");
		std::fs::write(dir.path().join("focus.md"), "# focus\nstay concise").unwrap();
		std::fs::write(dir.path().join("explore.md"), "# explore\nlook around").unwrap();

		let loaded = load_directives(&dir.path());
		assert_eq!(loaded.len(), 2);
		assert_eq!(loaded["focus"].content, "# focus\nstay concise");
		assert_eq!(loaded["explore"].content, "# explore\nlook around");
	}

	#[test]
	fn test_load_directives_skips_non_md_files() {
		let dir = TempDir::new("skip-non-md");
		std::fs::write(dir.path().join("focus.md"), "keep me").unwrap();
		std::fs::write(dir.path().join("README.txt"), "not a directive").unwrap();
		std::fs::write(dir.path().join("notes"), "no extension at all").unwrap();

		let loaded = load_directives(&dir.path());
		assert_eq!(loaded.len(), 1);
		assert!(loaded.contains_key("focus"));
	}

	#[test]
	fn test_load_directives_missing_dir_returns_builtins() {
		let missing = std::env::temp_dir().join("aphrodite-directives-test-does-not-exist");
		let loaded = load_directives(&missing);
		// Built-in directives are now returned as fallback when no
		// `directives/` directory exists on disk.
		assert!(
			!loaded.is_empty(),
			"missing dir should fall back to baked-in built-in directives"
		);
		assert!(
			loaded.contains_key("focus"),
			"built-in directives must include 'focus'"
		);
	}

	// ── Built-in directives: baked into the binary via include_str! ──
	#[test]
	fn test_loaded_builtins_contains_all_five() {
		let builtins = loaded_builtins();
		assert_eq!(builtins.len(), 5, "should have 5 baked-in directives");
		assert!(builtins.contains_key("focus"));
		assert!(builtins.contains_key("foresight"));
		assert!(builtins.contains_key("ccr-handling"));
		assert!(builtins.contains_key("cleanup"));
		assert!(builtins.contains_key("explore"));
	}

	#[test]
	fn test_load_directives_truncates_at_max_chars_with_ellipsis() {
		let dir = TempDir::new("truncate");
		let oversized = "x".repeat(MAX_DIRECTIVE_CHARS + 500);
		std::fs::write(dir.path().join("huge.md"), &oversized).unwrap();

		let loaded = load_directives(&dir.path());
		let content = &loaded["huge"].content;
		// MAX_DIRECTIVE_CHARS worth of 'x' plus the ellipsis marker.
		assert_eq!(content.chars().count(), MAX_DIRECTIVE_CHARS + 1);
		assert!(content.ends_with('…'));
	}

	#[test]
	fn test_load_directives_under_cap_is_not_truncated() {
		let dir = TempDir::new("under-cap");
		let small = "short directive body";
		std::fs::write(dir.path().join("small.md"), small).unwrap();

		let loaded = load_directives(&dir.path());
		assert_eq!(loaded["small"].content, small);
		assert!(!loaded["small"].content.ends_with('…'));
	}
}
