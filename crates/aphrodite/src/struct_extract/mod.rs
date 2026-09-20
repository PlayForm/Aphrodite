//! Code structure extractor - regex-based pattern matching per language.
//! Port of plugins/aphrodite/_core/struct.py
//!
//! Extracts function/struct/class signatures from source code with
//! a 300-char preview budget. Used by the preview engine to show
//! code structure in CCR markers like [code_rust:3fns 2structs].
//!
//! 1.5.0: atomized into a reverse-taxonomy tree (REFACTOR-PLAN-1.5.0):
//! `detect.rs` (language auto-detection), `format.rs` (byte-safe
//! truncation + signature formatting), `rust.rs`/`python.rs`/`go.rs`/
//! `js.rs` (one extractor per language), `tests.rs` (battery). This
//! facade re-exports the exact public names the rest of the crate
//! (and `bench/compression` via `lib.rs`) depends on.

use std::collections::HashMap;

pub mod detect;
pub mod format;
pub mod go;
pub mod js;
pub mod python;
pub mod rust;

#[cfg(test)]
mod tests;

use go::extract_go;
use js::extract_js;
use python::extract_python;
use rust::extract_rust;

/// Maximum total output in characters (preview budget).
pub(crate) const BUDGET:usize = 300;

/// Maximum length of a single signature line.
pub(crate) const MAX_SIG_LEN:usize = 60;

/// Maximum param string length before truncation.
pub(crate) const MAX_PARAMS_LEN:usize = 35;

pub(crate) use detect::auto_detect;
pub(crate) use format::{floor_boundary, sig, trunc_params};

/// Extract code structure from source content.
/// Auto-detects language from content prefixes.
/// Returns a map of category → list of short signature strings.
pub fn extract_code_structure(content:&str, language:&str) -> HashMap<String, Vec<String>> {
	let lang = if language.is_empty() { auto_detect(content) } else { language.to_string() };

	if lang.is_empty() {
		return HashMap::new();
	}

	let mut result:HashMap<String, Vec<String>> = HashMap::new();
	let mut budget = BUDGET as isize;

	match lang.as_str() {
		"rust" => extract_rust(content, &mut result, &mut budget),
		"python" => extract_python(content, &mut result, &mut budget),
		"go" => extract_go(content, &mut result, &mut budget),
		"js" | "ts" => extract_js(content, &mut result, &mut budget),
		_ => {},
	}

	result
}
