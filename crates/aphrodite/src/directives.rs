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
pub fn load_directives(dir:&PathBuf) -> HashMap<String, Directive> {
	let mut directives = HashMap::new();
	let Ok(entries) = std::fs::read_dir(dir) else {
		return directives;
	};
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
			serde_json::json!({
				"available": state.directives.keys().collect::<Vec<&String>>(),
				"active": &state.active_directives,
			})
		},
		"swap" => {
			if state.directives.contains_key(name) {
				state.active_directives = vec![name.to_string()];
				serde_json::json!({"swapped": name, "active": &state.active_directives})
			} else {
				serde_json::json!({"error": format!("unknown directive: {}", name)})
			}
		},
		"add" => {
			if state.directives.contains_key(name) && !state.active_directives.contains(&name.to_string()) {
				state.active_directives.push(name.to_string());
			}
			serde_json::json!({"active": &state.active_directives})
		},
		"remove" => {
			state.active_directives.retain(|d| d != name);
			serde_json::json!({"active": &state.active_directives})
		},
		"reset" => {
			state.active_directives.clear();
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
}
