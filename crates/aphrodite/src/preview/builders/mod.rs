//! Preview arms, one file per arm (1.5.0 REFACTOR-PLAN §3a) + the dispatch.
//! `build_preview` resolves the effective type and dispatches to the
//! per-arm builders; the cap is applied last on every path.

pub mod build;
pub mod cap;
pub mod code;
pub mod csv;
pub mod diff;
pub mod error;
pub mod generic;
pub mod git;
pub mod gitlog;
pub mod grep;
pub mod html;
pub mod json;
pub mod lint;
pub mod log;
pub mod ls;
pub mod markdown;
pub mod search;
pub mod table;
pub mod terminal;
pub mod test;
pub mod xml;
pub mod yaml;

use crate::preview::input::Input;
use crate::preview::state::PREVIEW_MAX_CHARS;
use crate::preview::r#type::resolve_effective_type;

/// Build a compact, human-readable preview string for compressed content,
/// shaped per content type (e.g. error/warning counts for build output,
/// +/- line counts for diffs, fn/struct counts for source code) so the LLM
/// gets a useful summary instead of a generic byte/line count wherever a
/// richer signal is available.
pub fn build_preview(type_str:&str, content:&str) -> String {
	let inp = Input::new(content).unwrap_or_else(|| Input::empty(content));
	let effective = resolve_effective_type(type_str, &inp);
	dispatch(effective.as_ref(), &inp)
}

/// Dispatch to the per-arm builder; the cap applies to EVERY path.
fn dispatch(effective:&str, inp:&Input<'_>) -> String {
	let preview = match effective {
		"build" | "build_output" | "build_error" => build::build_build_preview(inp),
		"diff" => diff::build_diff_preview(inp),
		// git status: staged/unstaged tallies by code + first few paths.
		"git" | "git_status" => git::build_git_status_preview(inp),
		// git log: commit count + first/last short hash and subject.
		"gitlog" | "git_log" => gitlog::build_gitlog_preview(inp),
		// directory listing: file/dir counts + top extensions.
		"ls" | "dir" | "directory" => ls::build_ls_preview(inp),
		// test output: pass/fail/ignored tallies + first failing test.
		"test" | "test_output" => test::build_test_preview(inp),
		// grep/ripgrep: hit count, files touched, first location.
		"grep" | "ripgrep" => grep::build_grep_preview(inp),
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh" | "code" => {
			code::build_code_preview(inp)
		},
		"search" => search::build_search_preview(inp),
		"html" => html::build_html_preview(inp),
		// ISSUE-11 residual #4: structured shapes that used to land on the
		// generic first-line arm (SHALLOW) - markdown tables, markdown docs,
		// yaml, xml, csv - each get a semantic arm with counts + a sample.
		"table" | "markdown_table" | "md_table" => table::build_table_preview(inp),
		"markdown" | "md" => markdown::build_markdown_preview(inp),
		"yaml" => yaml::build_yaml_preview(inp),
		"xml" => xml::build_xml_preview(inp),
		"csv" => csv::build_csv_preview(inp),
		"json_array" | "json" | "json_list" | "tool_output" => json::build_json_preview(inp),
		// `hooks::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace (F10).
		"terminal" => terminal::build_terminal_preview(inp),
		// Error output (Issue #11 WS2): surface the FIRST real error line.
		"error" => error::build_error_preview(inp),
		// Linter output (ruff/eslint/clippy/flake8).
		"linter" | "lint" => lint::build_lint_preview(inp),
		// Log output: error/failure signal wins, else the tail.
		"log" => log::build_log_preview(inp),
		// Plain-text / unrecognized fallback (residuals #1/#4 RC-D).
		_ => generic::build_generic_preview(effective, inp),
	};
	// ── Issue #11 WS4: enforce the configured `preview_max_chars` cap ──
	cap::apply_preview_cap(&preview, PREVIEW_MAX_CHARS.load(std::sync::atomic::Ordering::Relaxed) as usize)
}