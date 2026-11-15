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
	match content.trim_start().starts_with('{') {
		false => return None,
		true => {},
	}

	let v:Value = serde_json::from_str(content).ok()?;
	let obj = v.as_object()?;

	// ── Terminal: {"output":"...","exit_code":N,"error":null} ──
	// A non-empty `output` always wins (this arm is first); an empty one
	// falls through to the `error`/`success` arms instead of aborting
	// extraction entirely (F12) - a failed command with empty stdout is
	// exactly the case where the error string IS the payload.
	//
	// Terminal outputs are often short and don't trigger the headroom
	// classifier's build_output pattern, so the heuristics below give cargo
	// output, test runs and shell traces meaningful previews.
	let terminal = || {
		match (obj.get("output").and_then(|o| o.as_str()), obj.get("exit_code")) {
			(Some(output), Some(_exit)) if !output.is_empty() => {
				let ct:String = match output.contains("exit code:") || output.contains("Error:") {
					true => "terminal".into(),
					false => {
						match output.contains("   Compiling")
							|| output.contains("    Finished")
							|| output.contains("   Running")
							|| output.contains("test result:")
							|| output.contains("   Building")
							|| output.contains("   Installing")
							|| output.contains("warning:")
							|| output.contains("error[")
						{
							true => {
								match output.contains("error[") || output.contains("error: could not") {
									true => "build_error".into(),
									false => "build_output".into(),
								}
							},
							false => aphrodite::Preview::Detect::Type::Fn(output),
						}
					},
				};

				Some((output.to_string(), ct))
			},
			_ => None,
		}
	};

	// ── Patch / write_file: {"success":...,"diff":"...","error":"..."} ──
	let patch = || {
		match obj.get("diff").and_then(|d| d.as_str()) {
			Some(diff) if !diff.is_empty() => Some((diff.to_string(), aphrodite::Preview::Detect::Type::Fn(diff))),
			_ => None,
		}
	};

	let error = || {
		match obj.get("error").and_then(|m| m.as_str()) {
			Some(msg) if msg.starts_with("Found") && msg.contains("matches") => {
				Some((msg.to_string(), "text".to_string()))
			},
			Some(msg) if !msg.is_empty() && !msg.starts_with('{') && !msg.starts_with('[') => {
				Some((msg.to_string(), "text".to_string()))
			},
			_ => None,
		}
	};

	// ISSUE-11 WS1: the bare `{"success": true}` collapse (Some(("ok", "text")))
	// is deleted - it fired on ANY success-plus-one-key envelope and replaced the
	// whole payload with a 2-byte fragment, producing the reported
	// `[text:1L 2B | ok]`. Every genuine Hermes envelope is caught by the arms
	// above (output/exit_code, diff, error), so a success-only object is a user
	// payload and must fall through to detect_type / the caller hint. The
	// success-STRING arm survives only for SINGLE-key objects (e.g.
	// {"success": "wrote 3 files"}) - multi-key payloads like
	// {"success": "ok", "data": [...]} must not collapse to the word either.
	let success = || {
		match obj.get("success").and_then(|ok| ok.as_str()) {
			Some(msg) if obj.len() <= 1 && !msg.is_empty() && !msg.starts_with('{') => {
				Some((msg.to_string(), "text".to_string()))
			},
			_ => None,
		}
	};

	// ── Search: {"total_count":N,"matches":[...] or "matches_text", "truncated":bool} ──
	let search = || {
		match (obj.get("total_count").and_then(|c| c.as_u64()).is_some(), Search::Fn::Fn(obj)) {
			(true, Some((body, ct))) => Some((body, ct.to_string())),
			_ => None,
		}
	};

	// ── File read: {"content":"...","total_lines":N} ──
	// (read_file is usually essential-skipped, but handle it anyway.)
	let read = || {
		obj.get("content")
			.and_then(|c| c.as_str())
			.map(|text| (text.to_string(), aphrodite::Preview::Detect::Type::Fn(text)))
	};

	// ── Other Hermes tools: extract first meaningful string field ──
	// E.g. skill_view: {"name":"...","description":"..."}
	//       aphrodite_retrieve: {"found":true,"content":"..."}
	// ISSUE-11 WS1: only SINGLE-key objects collapse here. A multi-key payload
	// like {"result": "ok", "data": {...}} must preview as itself, not as the
	// fragment "ok" (the same bug class as the deleted success-bool collapse,
	// ungated by any size before).
	let priority_keys = ["description", "summary", "result", "message", "preview", "found"];
	let single = || {
		match obj.len() <= 1 {
			true => {
				priority_keys
					.iter()
					.filter_map(|key| obj.get(*key).and_then(|v| v.as_str()))
					.find(|s| !s.is_empty() && !s.starts_with('{') && !s.starts_with('['))
					.map(|s| (s.to_string(), "text".to_string()))
			},
			false => None,
		}
	};

	terminal()
		.or_else(patch)
		.or_else(error)
		.or_else(success)
		.or_else(search)
		.or_else(read)
		.or_else(single)
}
