//! The type-string to preview-arm dispatch table.
//!
//! Every content type the engine can classify has exactly one arm, and this match
//! is the only place that mapping exists. It is not a classifier and it holds no
//! rendering: an arm is chosen here and the arm itself decides what to show, so
//! adding a type is one line and reordering a priority is moving one line.

use crate::Preview::Input::Struct::Struct;
use super::{
	Build,
	CSV,
	Code,
	Diff,
	Error,
	Generic,
	Git,
	GitLog,
	Grep,
	HTML,
	JSON,
	Lint,
	Log,
	Ls,
	Markdown,
	Search,
	Table,
	Terminal,
	Test,
	XML,
	YAML,
};

/// Render the preview for an already-resolved effective type.
///
/// The caller has run type resolution, so `Effective` is trusted here - a type
/// that does not match any arm falls through to the generic arm rather than being
/// treated as an error, because an unknown type is a classification gap and not a
/// reason to render nothing. It is not responsible for the length cap: the entry
/// point applies that to every path.
pub(crate) fn Fn(Effective:&str, Inp:&Struct<'_>) -> String {
	match Effective {
		"build" | "build_output" | "build_error" => Build::Fn(Inp),
		"diff" => Diff::Fn(Inp),
		// git status: staged/unstaged tallies by code + first few paths.
		"git" | "git_status" => Git::Fn(Inp),
		// git log: commit count + first/last short hash and subject.
		"gitlog" | "git_log" => GitLog::Fn(Inp),
		// directory listing: file/dir counts + top extensions.
		"ls" | "dir" | "directory" => Ls::Fn(Inp),
		// test output: pass/fail/ignored tallies + first failing test.
		"test" | "test_output" => Test::Fn(Inp),
		// grep/ripgrep: hit count, files touched, first location.
		"grep" | "ripgrep" => Grep::Fn(Inp),
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh" | "code" => {
			Code::Fn(Inp)
		},
		"search" => Search::Fn(Inp),
		"html" => HTML::Fn(Inp),
		// Structured shapes that used to land on the generic first-line arm
		// (SHALLOW): markdown tables, markdown docs, yaml, xml, csv - each gets a
		// semantic arm with counts plus a sample.
		"table" | "markdown_table" | "md_table" => Table::Fn(Inp),
		"markdown" | "md" => Markdown::Fn(Inp),
		"yaml" => YAML::Fn(Inp),
		"xml" => XML::Fn(Inp),
		"csv" => CSV::Fn(Inp),
		"json_array" | "json" | "json_list" | "tool_output" => JSON::Fn(Inp),
		// `Hook::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace.
		"terminal" => Terminal::Fn(Inp),
		// Error output: surface the FIRST real error line.
		"error" => Error::Fn(Inp),
		// Linter output (ruff/eslint/clippy/flake8).
		"linter" | "lint" => Lint::Fn(Inp),
		// Log output: error/failure signal wins, else the tail.
		"log" => Log::Fn(Inp),
		// Plain-text / unrecognized fallback.
		_ => Generic::Fn(Effective, Inp),
	}
}
