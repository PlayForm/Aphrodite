//! The envelope chain: one matcher per wrapper shape, first match wins.
//!
//! Priority is position. `output` comes first because a terminal envelope that
//! also carries an `error` string must classify by its output when the output is
//! non-empty; the empty-output case falls through to the error matcher instead of
//! aborting, because a failed command with empty stdout IS the error text.

use serde_json::Value;

use crate::Tool::Unwrap::Matcher;

/// Extract the meaningful content from a Hermes envelope and reclassify it.
///
/// Returns `Some((content, ccr_type))` when the payload matches a known wrapper
/// shape, `None` when it does not (the caller then classifies the raw content
/// itself).
///
/// Two rules are load-bearing and were each a bug once:
/// - a bare `{"success": true}` is a USER payload, not a success envelope - the
///   old collapse to `("ok", "text")` produced `[text:1L 2B | ok]` and hid the
///   whole result;
/// - only SINGLE-key objects collapse to a priority string field, so
///   `{"result": "ok", "data": {...}}` previews as itself rather than as `ok`.
///
/// It is not a parser for arbitrary JSON: `[` arrays and non-objects return
/// `None` immediately, since the classifier already types an array correctly, and
/// the parse never runs for one.
///
/// This leaf composes; it decides nothing. Each shape is a named matcher under
/// `Matcher/`, so a new envelope is a new leaf plus one `or_else` line, and a
/// matcher that extracts with `?` and `filter` cannot evaluate a later
/// alternative eagerly the way a tuple scrutinee did.
pub(crate) fn Fn(Content:&str) -> Option<(String, String)> {
	let Parsed:Value = Content
		.trim_start()
		.starts_with('{')
		.then(|| serde_json::from_str(Content).ok())
		.flatten()?;
	let Object = Parsed.as_object()?;

	Matcher::Terminal::Fn::Fn(Object)
		.or_else(|| Matcher::Patch::Fn::Fn(Object))
		.or_else(|| Matcher::Error::Fn::Fn(Object))
		.or_else(|| Matcher::Success::Fn::Fn(Object))
		.or_else(|| Matcher::Search::Fn::Fn(Object))
		.or_else(|| Matcher::Read::Fn::Fn(Object))
		.or_else(|| Matcher::Single::Fn::Fn(Object))
}
