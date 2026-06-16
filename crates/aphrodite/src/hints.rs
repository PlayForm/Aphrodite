//! CCR Hinting — session-scoped memory annotations.
//!
//! A hint is a persistent mode switch injected into the agent's session.
//! Once set, it affects ALL future CCR operations until changed.
//!
//! Example: LLM calls `_ccr_hint="debug"` → agent now keeps errors
//! visible, shows full traces, disables log compression. The hint
//! persists across turns as part of the agent's working memory.

use std::sync::Mutex;

/// A hint injected into the agent's session memory.
/// Multiple hints compose — they stack, building the agent's mental model.
#[derive(Debug, Clone, PartialEq)]
pub enum Hint {
	/// Coding mode: keep signatures, extract structure, ×4 thresholds.
	Code(String),           // language hint: "rust", "python", etc.
	/// Debug mode: show full errors, keep traces, disable log compression.
	Debug,
	/// Review mode: show diffs, keep imports, prefer full content.
	Review,
	/// Compact mode: aggressive compression, minimal previews.
	Compact,
	/// Verbose mode: maximum preview, full structure, no truncation.
	Verbose,
	/// Custom mode: user-defined string that the pipeline can interpret.
	Custom(String),
}

/// Session-scoped hint stack.
/// Hints compose additively — setting "code_rust" + "debug" means
/// "keep Rust signatures AND show full error traces".
pub struct HintContext {
	hints: Mutex<Vec<Hint>>,
}

impl HintContext {
	pub fn new() -> Self {
		HintContext { hints: Mutex::new(Vec::new()) }
	}

	/// Add a hint to the session. Replaces any existing hint of the same kind.
	pub fn push(&self, hint: Hint) {
		let mut hints = self.hints.lock().unwrap_or_else(|e| e.into_inner());
		hints.retain(|h| std::mem::discriminant(h) != std::mem::discriminant(&hint));
		hints.push(hint);
	}

	/// Check if a specific mode is active.
	pub fn has(&self, predicate: impl Fn(&Hint) -> bool) -> bool {
		self.hints.lock().unwrap_or_else(|e| e.into_inner())
			.iter().any(predicate)
	}

	/// Get all active hints as strings (for marker metadata).
	pub fn to_metadata(&self) -> String {
		let hints = self.hints.lock().unwrap_or_else(|e| e.into_inner());
		hints.iter()
			.map(|h| match h {
				Hint::Code(lang) => format!("code={lang}"),
				Hint::Debug => "debug".to_string(),
				Hint::Review => "review".to_string(),
				Hint::Compact => "compact".to_string(),
				Hint::Verbose => "verbose".to_string(),
				Hint::Custom(s) => s.clone(),
			})
			.collect::<Vec<_>>()
			.join(";")
	}

	/// Parse a hint string from the LLM.
	/// Simple vocabulary: "code_rust", "debug", "review", "compact", "verbose"
	pub fn parse_and_push(&self, s: &str) {
		let hint = match s {
			"debug" => Hint::Debug,
			"review" => Hint::Review,
			"compact" => Hint::Compact,
			"verbose" => Hint::Verbose,
			s if s.starts_with("code_") => Hint::Code(s[5..].to_string()),
			s => Hint::Custom(s.to_string()),
		};
		self.push(hint);
	}

	/// Apply active hints to modify a preview string.
	/// Debug mode: show more. Compact mode: show less.
	pub fn apply_to_preview(&self, preview: &str, ct: &str) -> String {
		if self.has(|h| matches!(h, Hint::Debug | Hint::Verbose)) {
			// Debug/verbose: show more context
			preview.to_string()
		} else if self.has(|h| matches!(h, Hint::Compact)) {
			// Compact: truncate aggressively
			preview.chars().take(100).collect()
		} else {
			preview.to_string()
		}
	}

	/// Apply active hints to modify structure extraction depth.
	pub fn structure_depth(&self) -> usize {
		if self.has(|h| matches!(h, Hint::Debug | Hint::Verbose)) {
			5  // deeper extraction
		} else if self.has(|h| matches!(h, Hint::Compact)) {
			2  // shallow
		} else {
			3  // default
		}
	}
}

impl Default for HintContext {
	fn default() -> Self { Self::new() }
}
