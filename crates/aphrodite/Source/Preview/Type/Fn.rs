//! Type resolution contract.
//!
//! A caller's type hint and the content's actual shape can disagree, and every consumer must
//! resolve that disagreement the same way or the proxy and the dylib preview the same payload
//! differently. This function is the single source of truth for the precedence. It is not a
//! detector: it never inspects content beyond delegating to the semantic detector.

use std::borrow::Cow;

use crate::Preview::{Detect, Input::Struct::Struct, Line::Failure};

/// Resolve the effective type, and therefore the preview arm, from a caller hint and the
/// content's reality.
///
/// Precedence, in order:
///
/// 1. an explicit non-generic hint wins and is never upgraded;
/// 2. a generic bucket (`"text" | "terminal" | "log" | "" | "plain" | "tool_result"`) is
///    replaced by `Detect::SemanticType` when that recognises a shape, and otherwise keeps
///    the hint;
/// 3. `"build_output" | "build_error"` is upgraded to `"test"` ONLY when the run is clean - a
///    failing run keeps the build arm, because a failing test summary shown as build output
///    hides the failure;
/// 4. the terminal exit-code override folds in as an explicit terminal-path rule.
///
/// Rule 4 is a rule and not a fallback on purpose: making terminal universal would classify
/// every `Error:` line as terminal output.
///
/// It is not a re-classifier - it borrows the hint where it can, and only the semantic
/// detector's static strings replace it.
pub(crate) fn Fn<'a>(Hint:&'a str, Inp:&Struct<'_>) -> Cow<'a, str> {
	match Hint {
		"text" | "terminal" | "log" | "" | "plain" | "tool_result" => {
			Detect::SemanticType::Fn(Inp.raw).map_or(Cow::Borrowed(Hint), Cow::Borrowed)
		},
		"build_output" | "build_error" => {
			Detect::SemanticType::Fn(Inp.raw)
				.filter(|Detected| *Detected == "test" && !Inp.raw.lines().any(Failure::Fn))
				.map_or(Cow::Borrowed(Hint), Cow::Borrowed)
		},
		Other => Cow::Borrowed(Other),
	}
}
