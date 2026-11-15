//! Table-driven coverage of every `unwrap_hermes_result` branch.
//!
//! 01-F6/F12: this ~100-line heuristic had zero regression tests despite being
//! rewritten three times (bf181d7 -> 9e52762 -> 8f138c1), so every arm - including
//! the deliberate `None`s - is pinned here.

use crate::Tool::Unwrap;

/// Table row: label, input value, expected (status, kind).
type UnwrapCase<'a> = (&'a str, serde_json::Value, Option<(&'a str, &'a str)>);

#[test]
fn test_unwrap_hermes_result_table() {
	let cases:Vec<UnwrapCase<'_>> = vec![
		(
			"terminal output+exit_code",
			serde_json::json!({"output": "hello\n", "exit_code": 0}),
			Some(("hello\n", "text")),
		),
		(
			"terminal with exit code marker",
			serde_json::json!({"output": "boom\nexit code: 1\n", "exit_code": 1}),
			Some(("boom\nexit code: 1\n", "terminal")),
		),
		(
			"empty output falls through to error (F12)",
			serde_json::json!({"output": "", "exit_code": 1, "error": "command not found: cargp"}),
			Some(("command not found: cargp", "text")),
		),
		(
			"empty output, no error, no success -> None",
			serde_json::json!({"output": "", "exit_code": 1}),
			None,
		),
		(
			"diff",
			serde_json::json!({"success": true, "diff": "-a\n+b\n"}),
			Some(("-a\n+b\n", "text")),
		),
		(
			"error string",
			serde_json::json!({"error": "Found 3 matches"}),
			Some(("Found 3 matches", "text")),
		),
		(
			"success bool only -> None (no collapse)",
			serde_json::json!({"success": true}),
			None,
		),
		(
			"success bool with data payload",
			serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}),
			None,
		),
		(
			"success string with data payload",
			serde_json::json!({"success": "ok", "data": [1]}),
			None,
		),
		(
			"success string message",
			serde_json::json!({"success": "wrote 3 files"}),
			Some(("wrote 3 files", "text")),
		),
		(
			"search with matches",
			serde_json::json!({"total_count": 1, "matches": [{"path": "a.rs", "line": 3, "content": "fn x()"}]}),
			Some(("a.rs:3:fn x()", "search")),
		),
		(
			"search with matches_text (real Hermes shape)",
			serde_json::json!({
				"total_count": 19,
				"matches_format": "path-grouped",
				"matches_text": "src/a.rs\n  10: fn alpha()\nsrc/b.rs\n  3: struct Gamma\n",
			}),
			Some(("src/a.rs:10:fn alpha()\nsrc/b.rs:3:struct Gamma", "search")),
		),
		(
			"search without matches surfaces total as text",
			serde_json::json!({"total_count": 5, "truncated": true}),
			Some(("5 total (truncated)", "text")),
		),
		(
			"data object with total_count key only -> None",
			serde_json::json!({"total_count": 42, "items": [1, 2, 3]}),
			None,
		),
		(
			"file read content",
			serde_json::json!({"content": "hi", "total_lines": 1}),
			Some(("hi", "text")),
		),
		(
			"priority key single-key",
			serde_json::json!({"description": "does a thing"}),
			Some(("does a thing", "text")),
		),
		(
			"priority key multi-key -> None (no collapse)",
			serde_json::json!({"name": "skill", "description": "does a thing"}),
			None,
		),
		(
			"priority key multi-key data payload",
			serde_json::json!({"result": "ok", "data": {"web": [{"title": "x"}]}}),
			None,
		),
		("not an object", serde_json::json!(["a", "b"]), None),
		("plain string, not JSON", serde_json::json!("hello"), None),
		("no matching shape", serde_json::json!({"a": 1, "b": 2}), None),
	];

	cases.into_iter().for_each(|(label, input, expected)| {
		let content = input.to_string();
		let got = Unwrap::Fn::Fn(&content);

		match expected {
			Some((c, t)) => {
				let (gc, gt) = got.unwrap_or_else(|| panic!("{label}: expected Some, got None"));
				assert_eq!(gc, c, "{label}: content mismatch");
				assert_eq!(gt, t, "{label}: type mismatch");
			},
			None => assert!(got.is_none(), "{label}: expected None, got {got:?}"),
		}
	});
}
