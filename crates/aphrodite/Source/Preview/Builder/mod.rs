//! Preview arms, one file per arm (1.5.0 REFACTOR-PLAN §3a) + the dispatch.
//! `build_preview` resolves the effective type and dispatches to the
//! per-arm builders; the cap is applied last on every path.

pub mod Build;
pub mod Cap;
pub mod Code;
pub mod CSV;
pub mod Diff;
pub mod Error;
pub mod Generic;
pub mod Git;
pub mod GitLog;
pub mod Grep;
pub mod HTML;
pub mod JSON;
pub mod Lint;
pub mod Log;
pub mod Ls;
pub mod Markdown;
pub mod Search;
pub mod Table;
pub mod Terminal;
pub mod Test;
pub mod XML;
pub mod YAML;

use crate::Preview::{Input::Input, State::PREVIEW_MAX_CHARS, Type::resolve_effective_type};

/// Build a compact, human-readable preview string for compressed content,
/// shaped per content type (e.g. error/warning counts for build output,
/// +/- line counts for diffs, fn/struct counts for source code) so the LLM
/// gets a useful summary instead of a generic byte/line count wherever a
/// richer signal is available.
///
/// TOML-driven override (pre-Rust port): a registered
/// `[templates.preview.<family>.<ctype>]` format string renders first; the
/// result takes the same `preview_max_chars` cap as every other path. When
/// nothing is configured for this type (`render_preview` returns `None`) the
/// per-arm dispatch below runs unchanged.
pub fn build_preview(type_str:&str, content:&str) -> String {
	if let Some(rendered) = crate::Template::render_preview(type_str, content)
		&& !rendered.is_empty()
	{
		return Cap::apply_preview_cap(
			&rendered,
			PREVIEW_MAX_CHARS.load(std::sync::atomic::Ordering::Relaxed) as usize,
		);
	}
	let inp = Input::new(content).unwrap_or_else(|| Input::empty(content));
	let effective = resolve_effective_type(type_str, &inp);
	dispatch(effective.as_ref(), &inp)
}

/// Dispatch to the per-arm builder; the cap applies to EVERY path.
fn dispatch(effective:&str, inp:&Input<'_>) -> String {
	let preview = match effective {
		"build" | "build_output" | "build_error" => Build::build_build_preview(inp),
		"diff" => Diff::build_diff_preview(inp),
		// git status: staged/unstaged tallies by code + first few paths.
		"git" | "git_status" => Git::build_git_status_preview(inp),
		// git log: commit count + first/last short hash and subject.
		"gitlog" | "git_log" => GitLog::build_gitlog_preview(inp),
		// directory listing: file/dir counts + top extensions.
		"ls" | "dir" | "directory" => Ls::build_ls_preview(inp),
		// test output: pass/fail/ignored tallies + first failing test.
		"test" | "test_output" => Test::build_test_preview(inp),
		// grep/ripgrep: hit count, files touched, first location.
		"grep" | "ripgrep" => Grep::build_grep_preview(inp),
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh" | "code" => {
			Code::build_code_preview(inp)
		},
		"search" => Search::build_search_preview(inp),
		"html" => HTML::build_html_preview(inp),
		// ISSUE-11 residual #4: structured shapes that used to land on the
		// generic first-line arm (SHALLOW) - markdown tables, markdown docs,
		// yaml, xml, csv - each get a semantic arm with counts + a sample.
		"table" | "markdown_table" | "md_table" => Table::build_table_preview(inp),
		"markdown" | "md" => Markdown::build_markdown_preview(inp),
		"yaml" => YAML::build_yaml_preview(inp),
		"xml" => XML::build_xml_preview(inp),
		"csv" => CSV::build_csv_preview(inp),
		"json_array" | "json" | "json_list" | "tool_output" => JSON::build_json_preview(inp),
		// `Hook::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace (F10).
		"terminal" => Terminal::build_terminal_preview(inp),
		// Error output (Issue #11 WS2): surface the FIRST real error line.
		"error" => Error::build_error_preview(inp),
		// Linter output (ruff/eslint/clippy/flake8).
		"linter" | "lint" => Lint::build_lint_preview(inp),
		// Log output: error/failure signal wins, else the tail.
		"log" => Log::build_log_preview(inp),
		// Plain-text / unrecognized fallback (residuals #1/#4 RC-D).
		_ => Generic::build_generic_preview(effective, inp),
	};
	// ── Issue #11 WS4: enforce the configured `preview_max_chars` cap ──
	Cap::apply_preview_cap(&preview, PREVIEW_MAX_CHARS.load(std::sync::atomic::Ordering::Relaxed) as usize)
}
