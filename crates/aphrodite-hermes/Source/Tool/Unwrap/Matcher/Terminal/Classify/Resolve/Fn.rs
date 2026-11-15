//! Resolve a terminal output's kind, one decision at a time.

use crate::Tool::Unwrap::Matcher::Terminal::Classify::Kind;

/// The marker groups, in precedence order.
///
/// A failure trace outranks build output (a failed test run prints both
/// `test result:` and `error[`), and a compiler error outranks a clean build.
const FAILURE:&[&str] = &["exit code:", "Error:"];
const BUILD:&[&str] = &[
	"   Compiling",
	"    Finished",
	"   Running",
	"test result:",
	"   Building",
	"   Installing",
	"warning:",
	"error[",
];
const BUILD_ERROR:&[&str] = &["error[", "error: could not"];

/// Resolve `Output` into its kind, LAZILY.
///
/// Each group is scanned only when the groups above it missed, so a failure
/// trace costs one scan and a build costs two. A `match` over the three
/// predicates would scan the output three times before choosing an arm - the
/// eager-tuple shape this crate's rewrite removes - and `then` keeps the
/// compiler-error scan inside the build branch, where it belongs.
pub(crate) fn Fn(Output:&str) -> Kind::Enum {
	let Present = |Markers:&[&str]| Markers.iter().any(|Marker| Output.contains(*Marker));

	Present(FAILURE)
		.then_some(Kind::Enum::Failure)
		.or_else(|| Present(BUILD).then(|| Kind::Enum::Build { CompilerError:Present(BUILD_ERROR) }))
		.unwrap_or(Kind::Enum::Detected)
}
