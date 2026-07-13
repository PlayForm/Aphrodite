//! Conversational Directives — lightweight behavioral context for the LLM.
//!
//! Directives are short `.md` files that inject behavioral instructions into
//! the LLM's context via `pre_llm_call`. Unlike file content (which gets
//! compressed into CCR markers), directives are **always inline** — they're
//! compact enough to never need compression, and the engine injects them
//! directly into the LLM's context without any file-system round-trip or
//! retrieval step.
//!
//! Per profile, per user: `directives/*.md` files are swappable at runtime
//! via `aphrodite_directive("swap", "name")`.

use std::{
	collections::HashMap,
	path::PathBuf,
};

/// A loaded directive — name and content.
#[derive(Debug, Clone)]
pub struct Directive {
	pub name: String,
	pub content: String,
}

/// Maximum combined token budget for injected directive text (~50 tokens per
/// directive × 6 directives ≈ 300 tokens — well under the default 500).
pub const MAX_DIRECTIVE_CHARS: usize = 2000;

/// Load all `.md` files from a `directives/` directory.
/// Returns a map of name → Directive. Files without `.md` extension are
/// silently skipped.
pub fn load_directives(dir: &PathBuf) -> HashMap<String, Directive> {
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
			let trunc: String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
			format!("{}…", trunc)
		} else {
			content
		};
		directives.insert(name.to_string(), Directive {
			name: name.to_string(),
			content,
		});
	}
	directives
}

/// Build the context string injected into `pre_llm_call`.
/// Format:
/// ```
/// [directives: focus, foresight]
/// focus: stay concise, 1-2 tools/turn
/// foresight: anticipate next reads
/// ```
/// Returns empty string if no directives are active.
pub fn build_directive_context(
	all: &HashMap<String, Directive>,
	active: &[String],
) -> String {
	if active.is_empty() {
		return String::new();
	}
	let names: Vec<&str> = active.iter().map(|s| s.as_str()).collect();
	let mut out = format!("[directives: {}]\n", names.join(", "));
	for name in active {
		if let Some(d) = all.get(name) {
			let first_line = d.content.lines().next().unwrap_or(&d.content);
			out.push_str(&format!("{}: {}\n", d.name, first_line));
		}
	}
	out
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
		all.insert("focus".into(), Directive {
			name: "focus".into(),
			content: "stay concise\nuse 1-2 tools".into(),
		});
		let context = build_directive_context(&all, &["focus".into()]);
		assert!(context.contains("[directives: focus]"));
		assert!(context.contains("focus: stay concise"));
	}
}
