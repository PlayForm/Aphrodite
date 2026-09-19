//! Type resolution contract (1.5.0 REFACTOR-PLAN §5): the single source of
//! truth for hint-vs-content type precedence, extracted from the old inline
//! match in `build_preview` and the near-duplicate blocks in `hooks.rs`.

use std::borrow::Cow;

use crate::preview::detect::detect_semantic_type;
use crate::preview::input::Input;
use crate::preview::line::failure::is_failure_line;

/// Resolve the effective type/preview arm from a caller hint and content
/// reality:
///
/// 1. an explicit non-generic hint wins (never upgraded);
/// 2. generic buckets (`"text" | "terminal" | "log" | "" | "plain" |
///    "tool_result"`) → `detect_semantic_type(content).unwrap_or(hint)`;
/// 3. `"build_output" | "build_error"` → upgrade to `"test"` ONLY when the
///    run is clean (no failure line); a failing run keeps the build arm
///    (WS2 pin);
/// 4. (Phase 6) the terminal exit-code override (`hooks.rs:379-381`) folds
///    in as an explicit terminal-path rule - it must NOT become a universal
///    detector, or every `"Error:"` line would classify terminal.
pub(crate) fn resolve_effective_type<'a>(hint: &'a str, inp: &Input<'_>) -> Cow<'a, str> {
	match hint {
		"text" | "terminal" | "log" | "" | "plain" | "tool_result" => {
			detect_semantic_type(inp.raw).map(Cow::Borrowed).unwrap_or(Cow::Borrowed(hint))
		},
		"build_output" | "build_error" => match detect_semantic_type(inp.raw) {
			Some("test") if !inp.raw.lines().any(is_failure_line) => Cow::Borrowed("test"),
			_ => Cow::Borrowed(hint),
		},
		other => Cow::Borrowed(other),
	}
}
