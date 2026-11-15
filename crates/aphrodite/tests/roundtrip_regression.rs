//! Regression tests for the bench/proxy round-trip finding: retrieving
//! bench/corpus/wide_5k_keys.json (a 20,002-line document) was NOT
//! byte-identical - the 10,000-line pagination cap windowed it to the first
//! 10,000 lines with a `[lines 1-10000/20002]` header prepended, so the
//! returned body no longer hashed to its own marker hash
//! (bench/results/proxy-bench-20260915-221517.json:
//! "len 109011 vs 219014; first diff at char 0: '[' vs '{'").
//!
//! Fix: `limit: 0` now requests the FULL document (round-trip contract);
//! explicit limits are still clamped to the 10,000-line server cap (02-F5).

use std::path::PathBuf;

fn WideDocument() -> String {
	let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/corpus/wide_5k_keys.json");
	if let Ok(Content) = std::fs::read_to_string(&p) {
		return Content;
	}
	// Inline fallback: a >10,000-line document with the same shape.
	(0..20_002)
		.map(|i| format!("key_{i:05} = {}", i))
		.collect::<Vec<_>>()
		.join("\n")
}

#[test]
fn LimitZeroReturnsFullDocumentOver10000LinesByteIdentical() {
	let Content = WideDocument();
	assert!(
		Content.lines().count() > 10_000,
		"regression needs a document over the 10,000-line cap"
	);

	let (Got, Truncated) = aphrodite::Retrieve::Page::Fn::Fn(&Content, 0, 0).expect("paginate ok");
	assert_eq!(Got, Content, "limit:0 must return the exact original bytes");
	assert!(!Truncated, "limit:0 never truncates");
}

#[test]
fn FullDocumentRetrievalPreservesContentAddressingHash() {
	// Round-trip contract: the returned body hashes to the marker's own hash.
	let Content = WideDocument();
	let (Got, _) = aphrodite::Retrieve::Page::Fn::Fn(&Content, 0, 0).expect("paginate ok");

	let OriginalHash = headroom_core::ccr::compute_key(Content.as_bytes());
	let RetrievedHash = headroom_core::ccr::compute_key(Got.as_bytes());
	assert_eq!(
		OriginalHash, RetrievedHash,
		"retrieved content must hash to the same key as the stored content"
	);
}

#[test]
fn LargeDocumentRoundTripsThroughStoreAndExpand() {
	// End-to-end: store the wide document under its content hash and expand
	// it back - byte-identical, hence the hash contract holds.
	let Content = WideDocument();
	let hash = headroom_core::ccr::compute_key(Content.as_bytes());

	let mut State = aphrodite::State::AphroditeState::default();
	State.InlineStorePut(hash.clone(), Content.clone());

	let Expanded = aphrodite::Resolve::Expand::Fn(&mut State, &hash).expect("entry found");
	assert_eq!(Expanded, Content, "expand must return the exact original bytes");
	assert_eq!(headroom_core::ccr::compute_key(Expanded.as_bytes()), hash);
}

#[test]
fn ExplicitLimitOverCapStillTruncates() {
	// The 02-F5 safety cap still applies to explicit limits - only limit:0
	// means the full document.
	let Content = WideDocument();
	let (Got, Truncated) = aphrodite::Retrieve::Page::Fn::Fn(&Content, 0, 999_999).expect("paginate ok");
	assert!(Truncated);
	assert!(Got.starts_with("[lines 1-10000/"), "explicit limit must still window");
	assert!(Got.len() < Content.len());
}
