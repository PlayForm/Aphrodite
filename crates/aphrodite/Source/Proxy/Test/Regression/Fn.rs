//! Named regressions - each pins a defect that must not come back.

use std::sync::atomic::Ordering;

use crate::Proxy::Compress::Chat::Fn::Fn as CompressChatCompletion;
use crate::Proxy::Format::Fn::Fn as ProxyFormatCCROutput;
use crate::Proxy::Test::Support::Chat::Fn::ChatCompletionBody;
use crate::Proxy::Test::Support::State::Fn::TestStateWithCCR;

/// Corpus 07_tokens_saved.py: the AtomicU64 must actually be incremented
/// on the real compression path, not just exist unused in /stats.
#[test]
fn Regression07TokensSavedIncrementsOnCompress() {
	let Content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
	let body = ChatCompletionBody(&Content);
	let State = TestStateWithCCR();

	assert_eq!(State.TokensSaved.load(Ordering::Relaxed), 0);
	let Result = tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(CompressChatCompletion(&State, &body, None));
	assert!(Result.is_some(), "content above threshold must compress");
	assert!(
		State.TokensSaved.load(Ordering::Relaxed) > 0,
		"tokens_saved must be incremented by the real compression path"
	);
}

/// Corpus 11_should_compress.py: threshold gating must actually skip
/// compression below threshold, not compress unconditionally.
#[test]
fn Regression11BelowThresholdSkipsCompressionAndCounter() {
	let Content = "short reply below any threshold";
	let body = ChatCompletionBody(Content);
	let State = TestStateWithCCR();

	let Result = tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(CompressChatCompletion(&State, &body, None));
	assert!(Result.is_none(), "below-threshold content must not compress");
	assert_eq!(
		State.TokensSaved.load(Ordering::Relaxed),
		0,
		"no savings should be recorded when nothing was compressed"
	);
}

/// Corpus 13_engine_truncation.py: a CCR marker must never be truncated
/// mid-terminator by a preview-length budget - format_ccr_output's
/// output must always contain a complete `<<<CCR:hash|type|size>>>` line,
/// regardless of how long the preview or metadata are.
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
