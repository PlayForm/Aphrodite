//! Detection entry points: the headroom classifier wrapper (`detect_type`)
//! and the Aphrodite-side semantic detector (`detect_semantic_type`) as an
//! `Option::or_else` chain over the declarative detector pipeline.

use headroom_core::transforms;

use crate::preview::detectors;
use crate::preview::input::Input;

/// Detect the CCR content-type string for a blob (e.g. `source_code`, `build`,
/// `json_array`). Thin wrapper over the Headroom classifier so downstream
/// crates (aphrodite-hermes) don't need a direct headroom-core dependency.
pub fn detect_type(content:&str) -> String {
	transforms::content_detector::detect_content_type(content)
		.content_type
		.as_str()
		.to_string()
}

/// Aphrodite-side semantic detector for COMMON tool-output shapes the vendored
/// Headroom classifier flattens to bare `text`/`terminal`. Runs entirely in
/// Aphrodite's own layer (the fork boundary is preserved - vendor/ is never
/// touched); callers use it to OVERRIDE the classified type before building a
/// preview, the same override pattern `hooks::transform_terminal_output`
/// already uses for shell traces.
///
/// Returns `Some(type)` for a recognized shape (`json`, `test`, `diff`,
/// `code`, `table`, `markdown`, `yaml`, `html`, `xml`, `csv`, `build`, `git`,
/// `ls`, `grep`, `gitlog`), or `None` to leave the classifier's own verdict in
/// place. Detection is deliberately conservative (line-prefix / marker
/// patterns, majority votes) so a random paragraph is never mis-tagged.
/// Char-boundary safe and panic-free on empty/NUL/multibyte input.
///
/// Issue #11 residual #4 (RC-C): structured content (diffs, code, tables,
/// yaml/xml/csv, JSON objects, build logs) used to land on the generic
/// first-line arm - the detectors below give each shape its semantic arm.
///
/// Order is the contract: identical to the pre-split if-chain (json FIRST so a
/// JSON payload can never be hijacked by a marker substring inside its string
/// values; search/log/terminal are stubs returning None until Phase 6).
/// Adding a shape = one chain line + one `detectors/<shape>.rs` file;
/// reordering a priority = moving one line.
pub fn detect_semantic_type(content:&str) -> Option<&'static str> {
	let inp = Input::new(content)?;
	None
		.or_else(|| detectors::json::detect(&inp).then_some("json"))
		.or_else(|| detectors::test::detect(&inp).then_some("test"))
		.or_else(|| detectors::diff::detect(&inp).then_some("diff"))
		.or_else(|| detectors::code::detect(&inp).then_some("code"))
		.or_else(|| detectors::table::detect(&inp).then_some("table"))
		.or_else(|| detectors::markdown::detect(&inp).then_some("markdown"))
		.or_else(|| detectors::yaml::detect(&inp).then_some("yaml"))
		.or_else(|| detectors::xml::detect(&inp))                       // "html"/"xml"/None
		.or_else(|| detectors::csv::detect(&inp).then_some("csv"))
		.or_else(|| detectors::build::detect(&inp).then_some("build"))
		.or_else(|| detectors::git::detect(&inp).then_some("git"))
		.or_else(|| detectors::gitlog::detect(&inp).then_some("gitlog"))
		.or_else(|| detectors::grep::detect(&inp).then_some("grep"))
		.or_else(|| detectors::ls::detect(&inp).then_some("ls"))
		.or_else(|| detectors::search::detect(&inp).then_some("search"))   // Phase 6
		.or_else(|| detectors::log::detect(&inp).then_some("log"))         // Phase 6
		.or_else(|| detectors::terminal::detect(&inp).then_some("terminal")) // Phase 6
}