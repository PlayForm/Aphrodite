//! Aphrodite-side semantic detector.
//!
//! The vendored classifier flattens several common tool-output shapes to bare
//! `Text`/`terminal`, and a flattened type means the model is shown the generic
//! arm for content that has a much better one. This detector recovers those
//! shapes entirely in Aphrodite's own layer - the fork boundary is preserved and
//! `vendor/` is never touched. It is not a classifier replacement: it only
//! overrides the classifier's verdict for shapes it is certain about, and returns
//! `None` otherwise so the classifier's own opinion stands.

use crate::Preview::{Detector, Input::Struct::Struct};

/// Detect a common tool-output shape the classifier would flatten.
///
/// Returns `Some(type)` for a recognised shape (`JSON`, `test`, `Diff`, `Code`,
/// `Table`, `markdown`, `YAML`, `html`, `xml`, `csv`, `Build`, `Git`, `Ls`,
/// `grep`, `gitlog`, `search`, `Log`, `terminal`), or `None` to leave the
/// classifier's verdict in place.
///
/// The chain order is the contract, not an implementation detail: JSON runs first
/// so a JSON payload can never be hijacked by a marker-shaped substring inside one
/// of its string values, and the conservative shapes run last so they cannot
/// claim content a specific shape already recognised. Detection is deliberately
/// line-prefix and majority based, so a random paragraph is never mis-tagged, and
/// it is char-boundary safe and panic-free on empty, NUL or multibyte input.
///
/// Adding a shape is one chain line plus one `Detector/<Shape>.rs` file;
/// reordering a priority is moving one line.
pub fn Fn(Content:&str) -> Option<&'static str> {
	let Inp = Struct::new(Content)?;
	None.or_else(|| Detector::JSON::Fn(&Inp).then_some("json"))
		.or_else(|| Detector::Test::Fn(&Inp).then_some("test"))
		.or_else(|| Detector::Diff::Fn(&Inp).then_some("diff"))
		.or_else(|| Detector::Code::Fn(&Inp).then_some("code"))
		.or_else(|| Detector::Table::Fn(&Inp).then_some("table"))
		.or_else(|| Detector::Markdown::Fn(&Inp).then_some("markdown"))
		.or_else(|| Detector::YAML::Fn(&Inp).then_some("yaml"))
		.or_else(|| Detector::XML::Fn(&Inp)) // "html"/"xml"/None
		.or_else(|| Detector::CSV::Fn(&Inp).then_some("csv"))
		.or_else(|| Detector::Build::Fn(&Inp).then_some("build"))
		.or_else(|| Detector::Git::Fn(&Inp).then_some("git"))
		.or_else(|| Detector::GitLog::Fn(&Inp).then_some("gitlog"))
		.or_else(|| Detector::Grep::Fn(&Inp).then_some("grep"))
		.or_else(|| Detector::Ls::Fn(&Inp).then_some("ls"))
		.or_else(|| Detector::Search::Fn(&Inp).then_some("search"))
		.or_else(|| Detector::Log::Fn(&Inp).then_some("log"))
		.or_else(|| Detector::Terminal::Fn(&Inp).then_some("terminal"))
}
