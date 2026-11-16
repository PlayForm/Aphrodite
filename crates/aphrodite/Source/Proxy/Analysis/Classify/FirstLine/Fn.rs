//! Recognise a traceback, a build log or a lint dump by its first line.
//!
//! The preview pipeline deliberately leaves `error`/`linter`/`build_output` to
//! a caller-supplied hint, and this gateway path has no hint. Without this arm
//! an un-hinted traceback previews as `[text:... | Traceback (most recent call
//! last):]`, which is the misleading-preview bug class.
//!
//! The rules are an ordered table, not a decomposition of a value: each one is
//! a policy about the line, the first that matches wins, and the order is the
//! contract. `find` preserves it and stops at the first hit, so the later
//! predicates never run once an earlier one has claimed the content - which a
//! precomputed tuple of booleans matched afterwards would not.

use crate::Proxy::Analysis::Classify::FirstLine::Rule::Type::Type as Rule;

/// The three first-line families, in priority order.
const Rules:[Rule; 3] = [("error", Errorish), ("build_output", Buildish), ("linter", Linterish)];

/// Classify by the content's first line.
pub(crate) fn Fn(Content:&str) -> &'static str {
	let Line = Content.lines().next().unwrap_or("");

	Rules
		.iter()
		.find(|(_, Match)| Match(Line))
		.map(|(Name, _)| *Name)
		.unwrap_or("text")
}

/// A traceback, a panic or a compiler error headline.
fn Errorish(Line:&str) -> bool {
	["error", "Error", "ERROR", "Traceback", "panic", "thread '"]
		.iter()
		.any(|Prefix| Line.starts_with(Prefix))
}

/// A compiler or test-runner progress line.
fn Buildish(Line:&str) -> bool {
	["Compiling ", "   Compiling ", "running ", "test "]
		.iter()
		.any(|Prefix| Line.starts_with(Prefix))
		|| Line.contains("Finished")
}

/// A linter or type-checker finding.
fn Linterish(Line:&str) -> bool {
	let Prefixed = ["error[E", "error: ", "warning[", "warning: "]
		.iter()
		.any(|Prefix| Line.starts_with(Prefix));
	let Named = ["mypy", "clippy", "eslint", "tsc "].iter().any(|Needle| Line.contains(Needle));
	let Piped = Line.contains('|') && (Line.contains("error") || Line.contains("warning"));

	Prefixed || Named || Piped
}
