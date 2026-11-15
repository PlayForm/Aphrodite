//! Which family a detected content name belongs to.
//!
//! One table, so the metadata renderer and the threshold scaler can never
//! disagree about what "this is a git dump" means. The names are the ones
//! `Detect::Fn::Fn` and the preview pipeline actually produce; anything else is
//! `Other`, which both consumers treat as plain text - the behaviour the
//! string match's `_` arm had.

use crate::Proxy::ContentType::Family::Enum::Enum as ContentFamily;

/// Classify a detected content name.
pub(crate) fn Fn(Name:&'static str) -> ContentFamily {
	match Name {
		"code_rust" => ContentFamily::CodeRust,
		"code_python" => ContentFamily::CodePython,
		"code_go" => ContentFamily::CodeGo,
		"code_js" => ContentFamily::CodeJS,
		"code" => ContentFamily::Code,
		"error" => ContentFamily::Error,
		"diff" => ContentFamily::Diff,
		"git" => ContentFamily::Git,
		"build_output" => ContentFamily::BuildOutput,
		"log" => ContentFamily::Log,
		"linter" => ContentFamily::Linter,
		"json" => ContentFamily::Json,
		"tool_output" => ContentFamily::ToolOutput,
		"text" => ContentFamily::Text,
		_ => ContentFamily::Other,
	}
}
