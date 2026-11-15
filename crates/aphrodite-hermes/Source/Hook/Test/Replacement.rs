//! The replacement mapping: chain split, single marker, auto-expand.

use crate::Hook::Replacement::Fn::replacement_from;

// ── Chain-split invisibility contract: the LLM sees the natural
// per-segment CCR markers (joined, one per line), never the `summary`
// string that announces the mechanism. ──
#[test]
fn test_replacement_from_chain_split_joins_natural_markers() {
	let r = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"chain_split": true,
		"segments": 2,
		"summary": "[chain:2 segs | 200 orig → 90 markers]",
		"markers": [
			{
				"index": 0,
				"type": "terminal",
				"size": 120,
				"hash": "aaaa",
				"preview": "[terminal:120B] seg one",
				"marker": "<<<CCR:aaaa|terminal|120>>>\n[terminal:120B] seg one"
			},
			{
				"index": 1,
				"type": "build",
				"size": 80,
				"hash": "bbbb",
				"preview": "[build:80B] seg two",
				"marker": "<<<CCR:bbbb|build|80>>>\n[build:80B] seg two"
			}
		]
	});
	let out = replacement_from(&r, "test-session");
	let s = out.as_str().expect("chain_split must yield a string");
	// Natural markers, one per line.
	assert!(s.starts_with("<<<CCR:aaaa|terminal|120>>>"));
	assert!(s.contains("\n<<<CCR:bbbb|build|80>>>"));
	assert!(s.contains("[terminal:120B] seg one"));
	assert!(s.contains("[build:80B] seg two"));
	// The mechanism is invisible: no summary string, no 'chain' word.
	assert!(!s.contains("[chain:"));
	assert!(!s.contains("chain"));
	assert!(!s.contains("orig →"));
}

#[test]
fn test_replacement_from_non_chain_uses_single_marker() {
	let r = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": "cccc",
		"type": "text",
		"size": 42,
		"preview": "[text:42B] hi",
		"marker": "<<<CCR:cccc|text|42>>>\n[text:42B] hi"
	});
	let s = replacement_from(&r, "test-session");
	assert_eq!(s.as_str().unwrap(), "<<<CCR:cccc|text|42>>>\n[text:42B] hi");
}

// ── Auto-expand: the hook resolved the stored content inline; the LLM
// must receive the expanded string, never the marker. ──
#[test]
fn test_replacement_from_auto_expanded_returns_expanded() {
	let r = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"auto_expanded": true,
		"hash": "dddd",
		"type": "terminal",
		"size": 11,
		"preview": "[terminal:11B] hi",
		"marker": "<<<CCR:dddd|terminal|11>>>",
		"expanded": "hello world"
	});
	let s = replacement_from(&r, "test-session");
	assert_eq!(
		s.as_str().unwrap(),
		"hello world",
		"auto-expanded payload must return the resolved content"
	);
}

#[test]
fn test_replacement_from_auto_expanded_without_expanded_falls_back_to_marker() {
	// A payload flagging auto_expanded but carrying no usable `expanded`
	// string must fall through to the normal marker path, never produce
	// a null replacement that would erase the output.
	let r = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"auto_expanded": true,
		"hash": "dddd",
		"type": "terminal",
		"size": 11,
		"preview": "[terminal:11B] hi",
		"marker": "<<<CCR:dddd|terminal|11>>>"
	});
	let s = replacement_from(&r, "test-session");
	assert_eq!(s.as_str().unwrap(), "<<<CCR:dddd|terminal|11>>>");
}
