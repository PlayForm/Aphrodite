//! Generate the metadata block that accompanies a compressed marker.
//!
//! Metadata is the model's map of what it is no longer holding: the content
//! family, its size, its shape and the reason it was compressed. It is built
//! per content family because the useful facts differ - a build log's value is
//! its error lines, a JSON payload's is its key structure. A misleading block
//! is a bug, not cosmetic.

use super::{Build, Code, Diff, Error, Git, Go, JS, JSON, Linter, Log, Python, Rust, Text};

/// Generate structured metadata for CCR markers based on content type.
/// Returns pipe-safe key=value pairs (max 200 chars, | escaped to /).
pub(crate) fn Fn(Content:&str, Ct:&str) -> String {
	let LineCount = Content.lines().count();
	let mut Parts:Vec<String> = Vec::new();

	match Ct {
		"code_rust" => Rust::Fn(Content, LineCount, &mut Parts),
		"code_python" => Python::Fn(Content, LineCount, &mut Parts),
		"code_go" => Go::Fn(Content, LineCount, &mut Parts),
		"code_js" => JS::Fn(Content, LineCount, &mut Parts),
		"code" => Code::Fn(Content, LineCount, &mut Parts),
		"error" => Error::Fn(Content, &mut Parts),
		"diff" => Diff::Fn(Content, &mut Parts),
		"git" => Git::Fn(Content, &mut Parts),
		"build_output" => Build::Fn(Content, &mut Parts),
		"log" => Log::Fn(Content, LineCount, &mut Parts),
		"linter" => Linter::Fn(Content, LineCount, &mut Parts),
		"json" | "tool_output" => JSON::Fn(Content, &mut Parts),
		"text" => Text::Fn(Content, LineCount, &mut Parts),
		_ => Text::Fn(Content, LineCount, &mut Parts),
	}

	// Build final string: ;-separated key=value pairs (; safe within CCR marker's |
	// delimiters). Comma separates list items within values. Max 400 chars
	// (coding-tuned: enough for ~25 functions).
	let Result = Parts.join(";").replace('\n', " ").replace('\r', "");
	let Truncated:String = Result.chars().take(400).collect();
	Truncated.trim_end_matches([';', ' ', ',']).to_string()
}

// ── CCR output template (editable) ────────────────────────────────
