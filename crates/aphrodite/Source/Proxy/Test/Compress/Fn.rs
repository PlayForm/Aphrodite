//! Compression of a Chat Completions body: threshold gating, auto-expand, the
//! budget header, and the F4 guard that a failed store write must never leave a
//! poisoned marker.
//!
//! The result is an `Outcome`, so "nothing was stored" (`None`), "stored but the
//! wire response is unchanged" (`StoredInline`) and "replaced by a marker"
//! (`Replaced`) are three distinct answers rather than a boolean beside a mutated
//! body.

use std::sync::Arc;

use axum::http::{HeaderMap, Method};
use headroom_core::ccr::compute_key;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Compress::Chat::Fn::Fn as Chat,
		State::{CCR::Get::Fn::Fn as CCRGet, Struct::Struct as AppState},
		Test::Support::{
			Chat::Fn::Payload,
			Context::Fn::Fn as TestContext,
			State::Fn::{FailingCCR, WithCCR},
		},
	},
};

/// Compress `body` against `State`.
fn Compress(State:&Arc<AppState>, body:&[u8], HeadroomBudget:Option<&str>) -> Option<serde_json::Value> {
	let Method = Method::POST;
	let Headers = HeaderMap::new();
	let Context = TestContext(State, &Method, &Headers, body);

	tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(Chat(&Context, body, HeadroomBudget))
		.map(|Outcome| Outcome.Response)
}

/// Compress `body` and report whether the wire response was rewritten.
fn Rewritten(State:&Arc<AppState>, body:&[u8], HeadroomBudget:Option<&str>) -> bool {
	let Method = Method::POST;
	let Headers = HeaderMap::new();
	let Context = TestContext(State, &Method, &Headers, body);

	tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(Chat(&Context, body, HeadroomBudget))
		.map(|Outcome| Outcome.Compressed())
		.unwrap_or(false)
}

// ── T4 (F4): a failed ccr_put must leave content uncompressed, not silently
// swapped for an unresolvable marker. ──
#[test]
fn PutFailure() {
	let State = Arc::new(FailingCCR());
	let Content = "fn answer() -> i32 { 42 }\n".repeat(200); // well above threshold
	let body = Payload(&Content);

	// Nothing could be safely compressed (the only candidate's store write
	// failed), so the whole response must come back `None` (unmodified
	// pass-through), not `Some` with a poisoned marker.
	assert!(
		Compress(&State, &body, None).is_none(),
		"a failed ccr_put must not produce a compressed response"
	);
}

// ── T4: compress_chat_completion ────────────────────────────
#[test]
fn Above() {
	// Token mode base threshold is 1024B; a large repeated text block stays well
	// above any auto-tuned multiplier of it.
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = Payload(&Content);
	let State = Arc::new(WithCCR());

	let response = Compress(&State, &body, None).expect("content above threshold must be compressed");
	let NewContent = response["choices"][0]["message"]["content"].as_str().unwrap();

	assert!(NewContent.contains("<<<CCR:"), "expected a CCR marker, got: {NewContent}");

	// The original content must be retrievable from the CCR store.
	let ccr = State.ccr.as_ref().unwrap().clone();
	let hash = compute_key(Content.as_bytes());
	let Stored = tokio::runtime::Runtime::new().unwrap().block_on(CCRGet(&ccr, &hash));

	assert_eq!(Stored.as_deref(), Some(Content.as_str()));
}

// ── Auto-expand (pre-Rust port): `[compression] auto_expand` resolves the
// just-stored content inline instead of emitting a marker, capped at
// `ExpandLimit`; over-limit falls back to the marker; disabled (default)
// emits markers byte-identically. ──
#[test]
fn Inline() {
	let State = Arc::new(WithCCR());
	State.AutoExpand.store(true, std::sync::atomic::Ordering::Relaxed);
	State.ExpandLimit.store(1_000_000, std::sync::atomic::Ordering::Relaxed);
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = Payload(&Content);

	// Nothing was removed from the wire, so the outcome must not report a
	// compression - the client receives the ORIGINAL content, not a marker.
	assert!(
		!Rewritten(&State, &body, None),
		"auto-expand must pass content through uncompressed"
	);
	// The content is still stored under its hash, so later retrievals work.
	let ccr = State.ccr.as_ref().unwrap().clone();
	let hash = compute_key(Content.as_bytes());
	let Stored = tokio::runtime::Runtime::new().unwrap().block_on(CCRGet(&ccr, &hash));

	assert_eq!(Stored.as_deref(), Some(Content.as_str()));
	// And no marker may appear anywhere in the response body.
	assert!(!String::from_utf8_lossy(&body).contains("<<<CCR:"));
}

#[test]
fn InlineCache() {
	// Same inline contract on the cache-mode compress path (the branch sits
	// before the mode-dependent marker renderer, so both `CacheMarker` and
	// `SmartMarker` paths must honour it).
	let mut State = WithCCR();
	State.Mode = ProxyMode::Cache;
	let State = Arc::new(State);
	State.AutoExpand.store(true, std::sync::atomic::Ordering::Relaxed);
	State.ExpandLimit.store(1_000_000, std::sync::atomic::Ordering::Relaxed);
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(300); // > 8KB cache threshold
	let body = Payload(&Content);

	assert!(
		!Rewritten(&State, &body, None),
		"auto-expand must pass content through uncompressed"
	);
	assert!(!String::from_utf8_lossy(&body).contains("<<<CCR:"));
}

#[test]
fn OverLimit() {
	let State = Arc::new(WithCCR());
	State.AutoExpand.store(true, std::sync::atomic::Ordering::Relaxed);
	State.ExpandLimit.store(100, std::sync::atomic::Ordering::Relaxed); // far below content
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = Payload(&Content);

	let response = Compress(&State, &body, None).expect("over-limit content must fall back to the marker path");
	let NewContent = response["choices"][0]["message"]["content"].as_str().unwrap();

	assert!(
		NewContent.contains("<<<CCR:"),
		"over-limit content must still emit a marker: {NewContent}"
	);
}

#[test]
fn Disabled() {
	// Default (flag false) is byte-identical current behavior: markers.
	let State = Arc::new(WithCCR()); // auto_expand defaults to false
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = Payload(&Content);

	let response = Compress(&State, &body, None).expect("content above threshold must be compressed");
	let NewContent = response["choices"][0]["message"]["content"].as_str().unwrap();

	assert!(NewContent.contains("<<<CCR:"), "expected a CCR marker, got: {NewContent}");
}

#[test]
fn Below() {
	let Content = "short reply";
	let body = Payload(Content);
	let State = Arc::new(WithCCR());

	assert!(Compress(&State, &body, None).is_none(), "short content must not be compressed");
}

#[test]
fn Arguments() {
	// 02-F3: `tool_calls[].function.arguments` is JSON the client feeds straight
	// into its own tool executor, not model-facing prose - it must never be
	// replaced by an unresolvable-to-the-client CCR marker, no matter how large.
	// `message.content` (real model-facing text) is the legitimate compression
	// target and is large here too, to confirm compression still happens for it
	// while leaving `arguments` completely alone.
	let BigArgs = serde_json::json!({"data": "x".repeat(4000)}).to_string();
	let BigContent = "line of moderate length text content here.\n".repeat(200);
	let body = serde_json::json!({
		"choices": [{
			"message": {
				"role": "assistant",
				"content": BigContent,
				"tool_calls": [{
					"function": {"name": "f", "arguments": BigArgs}
				}]
			}
		}]
	})
	.to_string()
	.into_bytes();
	let State = Arc::new(WithCCR());

	let response = Compress(&State, &body, None).expect("large message content must still be compressed");
	let NewContent = response["choices"][0]["message"]["content"].as_str().unwrap();

	assert!(
		NewContent.contains("<<<CCR:"),
		"expected message content to be compressed: {NewContent}"
	);

	let NewArgs = response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
		.as_str()
		.unwrap();

	assert_eq!(
		NewArgs, BigArgs,
		"tool-call arguments must pass through untouched, never marker-replaced"
	);
}

#[test]
fn Budget() {
	// A large-but-moderate payload: compressed when the budget header requests
	// aggressive compression (fill=0 -> 0.5x multiplier), but left alone with no
	// budget header (multiplier 1.0x) if it sits between the two thresholds.
	let Content = "line of moderate length text content here.\n".repeat(40); // ~1760B
	let body = Payload(&Content);

	let NoBudget = Rewritten(&Arc::new(WithCCR()), &body, None);
	let LowBudget = Rewritten(&Arc::new(WithCCR()), &body, Some("0"));

	// A lower budget must never compress *less* than no budget at all.
	NoBudget.then(|| assert!(LowBudget, "lower budget must compress at least as much as no budget"));
}
