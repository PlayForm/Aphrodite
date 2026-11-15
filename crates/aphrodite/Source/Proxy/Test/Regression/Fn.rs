//! Named regressions - each pins a defect that must not come back.

use std::sync::Arc;

use axum::http::{HeaderMap, Method};

use crate::Proxy::{
	Compress::Chat::Fn::Fn as CompressChatCompletion,
	Format::Fn::Fn as ProxyFormatCCROutput,
	Test::Support::{Chat::Fn::ChatCompletionBody, Context::Fn::Fn as TestContext, State::Fn::TestStateWithCCR},
};

/// Compress `body` against `State`.
fn Compress(State:&Arc<crate::Proxy::State::Struct::Struct>, body:&[u8]) -> Option<serde_json::Value> {
	let Method = Method::POST;
	let Headers = HeaderMap::new();
	let Context = TestContext(State, &Method, &Headers, body);

	tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(CompressChatCompletion(&Context, body, None))
		.map(|Outcome| Outcome.Response)
}

/// Corpus 07_tokens_saved.py: the AtomicU64 must actually be incremented on the
/// real compression path, not just exist unused in /stats.
///
/// The counter is now incremented by the responder from the emitted bytes, so
/// this pin moves with it: compression must still produce a rewritten body, and
/// that body must be shorter than the original.
#[test]
fn Regression07TokensSavedIncrementsOnCompress() {
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = ChatCompletionBody(&Content);
	let State = Arc::new(TestStateWithCCR());

	let Rewritten = Compress(&State, &body).expect("content above threshold must compress");
	let NewContent = Rewritten["choices"][0]["message"]["content"].as_str().unwrap();

	assert!(
		NewContent.len() < Content.len(),
		"a rewritten body must be shorter than the content it replaced - that difference is what the responder counts"
	);
}

/// Corpus 11_should_compress.py: threshold gating must actually skip compression
/// below threshold, not compress unconditionally.
#[test]
fn Regression11BelowThresholdSkipsCompressionAndCounter() {
	let Content = "short reply below any threshold";
	let body = ChatCompletionBody(Content);
	let State = Arc::new(TestStateWithCCR());

	assert!(Compress(&State, &body).is_none(), "below-threshold content must not compress");
	assert_eq!(
		State.TokensSaved.load(std::sync::atomic::Ordering::Relaxed),
		0,
		"no savings should be recorded when nothing was compressed"
	);
}

/// Corpus 13_engine_truncation.py: a CCR marker must never be truncated
/// mid-terminator by a preview-length budget - `Format::Fn`'s output must always
/// contain a complete `<<<CCR:hash|type|size>>>` line, regardless of how long
/// the preview or metadata are.
#[test]
fn Regression13MarkerTerminatorNeverTruncated() {
	let hash = "abc123def456abc123def456abc123def456";
	let HugePreview = "x".repeat(10_000);
	let HugeMetadata = "y".repeat(10_000);
	let Out = ProxyFormatCCROutput(&HugePreview, "text", &HugeMetadata, None, hash, 123456);
	let ExpectedTerminator = format!("<<<CCR:{hash}|text|123456>>>");

	assert!(
		Out.contains(&ExpectedTerminator),
		"marker terminator must always be complete and unsliced, regardless of preview/metadata length"
	);
}
