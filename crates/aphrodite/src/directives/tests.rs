//! Test battery for the directives module (moved verbatim from the former
//! monolithic directives.rs).
//!
//! `field_reassign_with_default` triggers on the verbatim
//! `Default::default()` + `s.turn_counter = N` setup pattern inherited from
//! the monolithic file - pre-existing in the original code, preserved as-is
//! for the frozen test contract.
#![allow(clippy::field_reassign_with_default)]

use std::collections::HashMap;

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
// delegate to - cover all six actions plus an unknown one directly. ──
#[test]
fn test_handle_action_all_actions_and_unknown() {
	let mut state = crate::state::AphroditeState::default();
	state
		.directives
		.insert("focus".into(), Directive { name:"focus".into(), content:"stay focused".into() });
	state
		.directives
		.insert("lazy".into(), Directive { name:"lazy".into(), content:"defer work".into() });

	let r = handle_action(&mut state, "list", "");
	assert_eq!(r["available"], serde_json::json!(["focus", "lazy"]));
	assert_eq!(r["active"], serde_json::json!([]));

	let r = handle_action(&mut state, "swap", "focus");
	assert_eq!(r["swapped"], "focus");
	assert_eq!(state.active_directives, vec!["focus".to_string()]);

	let r = handle_action(&mut state, "swap", "nonexistent");
	assert!(r["error"].as_str().unwrap().contains("unknown directive"));

	let r = handle_action(&mut state, "remove", "focus");
	assert_eq!(r["active"], serde_json::json!([]));

	// ── "load" activates a known directive and returns {loaded, active};
	// loading an already-active directive is idempotent and still
	// reports success (no error). ──
	let r = handle_action(&mut state, "load", "lazy");
	assert_eq!(r["loaded"], "lazy");
	assert_eq!(r["active"], serde_json::json!(["lazy"]));
	let r = handle_action(&mut state, "load", "lazy");
	assert_eq!(r["loaded"], "lazy", "re-loading must be idempotent");
	// "load" errors on an unknown directive (unlike "add", which is silent).
	let r = handle_action(&mut state, "load", "ghost");
	assert!(r["error"].as_str().unwrap().contains("unknown directive"));

	let r = handle_action(&mut state, "add", "focus");
	assert_eq!(r["active"], serde_json::json!(["lazy", "focus"]));

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

// ── Issue #6: an existing-but-empty directives directory is an
// INTENTIONAL empty set - empty map, NO built-in fallback (a missing
// directory still falls back to builtins, see
// test_load_directives_missing_dir_returns_builtins). ──
#[test]
fn test_load_directives_existing_empty_dir_returns_empty_map() {
	let dir = TempDir::new("empty-dir");
	// Non-.md files must not count as a usable directive either.
	std::fs::write(dir.path().join("README.txt"), "not a directive").unwrap();

	let loaded = load_directives(&dir.path());
	assert!(
		loaded.is_empty(),
		"an existing empty directives dir must yield an empty map, not builtins"
	);
}

// ── Issue #6: an empty .md file is a *readable* .md file, so it wins
// over builtins and loads as a directive with empty content. ──
#[test]
fn test_load_directives_empty_md_file_loads_empty_content() {
	let dir = TempDir::new("empty-md");
	std::fs::write(dir.path().join("empty.md"), "").unwrap();

	let loaded = load_directives(&dir.path());
	assert_eq!(loaded.len(), 1, "the empty .md file must load, not builtins");
	assert!(loaded.contains_key("empty"));
	assert_eq!(loaded["empty"].content, "");
}

// ── Issue #6: whitespace-only .md content loads verbatim (no trimming
// beyond the size cap). ──
#[test]
fn test_load_directives_whitespace_only_loads_verbatim() {
	let dir = TempDir::new("whitespace-md");
	let ws = "   \n\t\n  \n";
	std::fs::write(dir.path().join("ws.md"), ws).unwrap();

	let loaded = load_directives(&dir.path());
	assert_eq!(loaded.len(), 1);
	assert_eq!(loaded["ws"].content, ws, "whitespace-only content must load verbatim");
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
	assert!(loaded.contains_key("focus"), "built-in directives must include 'focus'");
}

// ── Built-in directives: baked into the binary via include_str! ──
#[test]
fn test_loaded_builtins_contains_all_seven() {
	let builtins = loaded_builtins();
	assert_eq!(builtins.len(), 7, "should have 7 baked-in directives");
	assert!(builtins.contains_key("focus"));
	assert!(builtins.contains_key("foresight"));
	assert!(builtins.contains_key("ccr-handling"));
	assert!(builtins.contains_key("cleanup"));
	assert!(builtins.contains_key("explore"));
	assert!(builtins.contains_key("lazy"));
	assert!(builtins.contains_key("lazy-eval"));
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
