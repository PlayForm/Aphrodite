//! The envelope chain: one arm per wrapper shape, first match wins.
//!
//! Priority is position. `output` comes first because a terminal envelope that
//! also carries an `error` string must classify by its output when the output is
//! non-empty; the empty-output case falls through to the error arm instead of
//! aborting, because a failed command with empty stdout IS the error text.

use serde_json::Value;

use crate::Tool::Unwrap::Search;

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
/// `None` immediately, since the classifier already types an array correctly.
pub(crate) fn Fn(content:&str) -> Option<(String, String)> {
	// Only attempt unwrapping if the content looks like a JSON object.
	if !content.trim_start().starts_with('{') {
		return None;
	}
	let v:Value = serde_json::from_str(content).ok()?;
	let obj = v.as_object()?;

	// ── Terminal: {"output":"...","exit_code":N,"error":null} ──
	// A non-empty `output` always wins (checked first, below); an empty one
	// falls through to the `error`/`success` branches instead of aborting
	// extraction entirely (F12) - a failed command with empty stdout is
	// exactly the case where the error string IS the payload.
	if let (Some(output), Some(_exit)) = (obj.get("output").and_then(|o| o.as_str()), obj.get("exit_code"))
		&& !output.is_empty()
	{
		// Terminal outputs are often short and don't trigger the headroom
		// classifier's build_output pattern. Add explicit heuristics so
		// cargo output, test runs, and shell traces get meaningful previews.
		let ct:String = if output.contains("exit code:") || output.contains("Error:") {
			"terminal".into()
		} else if output.contains("   Compiling")
			|| output.contains("    Finished")
			|| output.contains("   Running")
			|| output.contains("test result:")
			|| output.contains("   Building")
			|| output.contains("   Installing")
			|| output.contains("warning:")
			|| output.contains("error[")
		{
			if output.contains("error[") || output.contains("error: could not") {
				"build_error".into()
			} else {
				"build_output".into()
			}
		} else {
			aphrodite::DetectType(output)
		};
		return Some((output.to_string(), ct));
	}

	// ── Patch / write_file: {"success":...,"diff":"...","error":"..."} ──
	if let Some(diff) = obj.get("diff").and_then(|d| d.as_str())
		&& !diff.is_empty()
	{
		return Some((diff.to_string(), aphrodite::DetectType(diff)));
	}
	if let Some(msg) = obj.get("error").and_then(|m| m.as_str()) {
		if msg.starts_with("Found") && msg.contains("matches") {
			return Some((msg.to_string(), "text".to_string()));
		}
		if !msg.is_empty() && !msg.starts_with('{') && !msg.starts_with('[') {
			return Some((msg.to_string(), "text".to_string()));
		}
	}
	if let Some(ok) = obj.get("success") {
		// ISSUE-11 WS1: the bare `{"success": true}` collapse
		// (Some(("ok", "text"))) is deleted - it fired on ANY success-plus-
		// one-key envelope and replaced the whole payload with a 2-byte
		// fragment, producing the reported `[text:1L 2B | ok]`. Every
		// genuine Hermes envelope is caught by the branches above
		// (output/exit_code, diff, error), so a success-only object is a
		// user payload: let it fall through to detect_type / the caller
		// hint. The success-STRING branch survives only for SINGLE-key
		// objects (e.g. {"success": "wrote 3 files"}) - multi-key payloads
		// like {"success": "ok", "data": [...]} must not collapse to the
		// word either.
		if let Some(msg) = ok.as_str()
			&& obj.len() <= 1
			&& !msg.is_empty()
			&& !msg.starts_with('{')
		{
			return Some((msg.to_string(), "text".to_string()));
		}
	}

	// ── Search: {"total_count":N,"matches":[...] or "matches_text", "truncated":bool} ──
	if obj.get("total_count").and_then(|c| c.as_u64()).is_some()
		&& let Some((body, ct)) = Search::Fn::Fn(obj)
	{
		return Some((body, ct.to_string()));
	}

	// ── File read: {"content":"...","total_lines":N} ──
	// (read_file is usually essential-skipped, but handle it anyway.)
	if let Some(text) = obj.get("content").and_then(|c| c.as_str()) {
		return Some((text.to_string(), aphrodite::DetectType(text)));
	}

	// ── Other Hermes tools: extract first meaningful string field ──
	// E.g. skill_view: {"name":"...","description":"..."}
	//       aphrodite_retrieve: {"found":true,"content":"..."}
	// ISSUE-11 WS1: only SINGLE-key objects collapse here. A multi-key
	// payload like {"result": "ok", "data": {...}} must preview as
	// itself, not as the fragment "ok" (the same bug class as the
	// deleted success-bool collapse, ungated by any size before).
	let priority_keys = ["description", "summary", "result", "message", "preview", "found"];
	if obj.len() <= 1 {
		for key in &priority_keys {
			if let Some(s) = obj.get(*key).and_then(|v| v.as_str())
				&& !s.is_empty()
				&& !s.starts_with('{')
				&& !s.starts_with('[')
			{
				return Some((s.to_string(), "text".to_string()));
			}
		}
	}

	None
}
