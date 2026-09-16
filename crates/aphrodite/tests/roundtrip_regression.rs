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

fn wide_document() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/corpus/wide_5k_keys.json");
    if let Ok(content) = std::fs::read_to_string(&p) {
        return content;
    }
    // Inline fallback: a >10,000-line document with the same shape.
    (0..20_002).map(|i| format!("key_{i:05} = {}", i)).collect::<Vec<_>>().join("\n")
}

#[test]
fn limit_zero_returns_full_document_over_10000_lines_byte_identical() {
    let content = wide_document();
    assert!(
        content.lines().count() > 10_000,
        "regression needs a document over the 10,000-line cap"
    );

    let (got, truncated) = aphrodite::retrieve::paginate(&content, 0, 0).expect("paginate ok");
    assert_eq!(got, content, "limit:0 must return the exact original bytes");
    assert!(!truncated, "limit:0 never truncates");
}

#[test]
fn full_document_retrieval_preserves_content_addressing_hash() {
    // Round-trip contract: the returned body hashes to the marker's own hash.
    let content = wide_document();
    let (got, _) = aphrodite::retrieve::paginate(&content, 0, 0).expect("paginate ok");

    let original_hash = headroom_core::ccr::compute_key(content.as_bytes());
    let retrieved_hash = headroom_core::ccr::compute_key(got.as_bytes());
    assert_eq!(
        original_hash, retrieved_hash,
        "retrieved content must hash to the same key as the stored content"
    );
}

#[test]
fn large_document_round_trips_through_store_and_expand() {
    // End-to-end: store the wide document under its content hash and expand
    // it back - byte-identical, hence the hash contract holds.
    let content = wide_document();
    let hash = headroom_core::ccr::compute_key(content.as_bytes());

    let mut state = aphrodite::state::AphroditeState::default();
    state.inline_store_put(hash.clone(), content.clone());

    let expanded = aphrodite::resolve::expand(&mut state, &hash).expect("entry found");
    assert_eq!(expanded, content, "expand must return the exact original bytes");
    assert_eq!(headroom_core::ccr::compute_key(expanded.as_bytes()), hash);
}

#[test]
fn explicit_limit_over_cap_still_truncates() {
    // The 02-F5 safety cap still applies to explicit limits - only limit:0
    // means the full document.
    let content = wide_document();
    let (got, truncated) = aphrodite::retrieve::paginate(&content, 0, 999_999).expect("paginate ok");
    assert!(truncated);
    assert!(got.starts_with("[lines 1-10000/"), "explicit limit must still window");
    assert!(got.len() < content.len());
}