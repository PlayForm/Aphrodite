//! Regression tests for the root workspace's `serde_json` feature
//! unification (report 08 F2).
//!
//! The root `Cargo.toml` pins `preserve_order` + `arbitrary_precision` on
//! `serde_json` specifically because `vendor/headroom`'s `headroom-core`
//! requires them and Cargo unifies features across a single dependency
//! graph - if the root workspace ever drops (or a `cargo upgrade` pass
//! regenerates) that feature list, headroom-core silently gets built
//! without them. That happened once already (see
//! `docs/HEADROOM-FORK-DIFF.md`'s 2026-07-11 merge section): object keys
//! came out alphabetically sorted instead of insertion-ordered, breaking
//! SmartCrusher's anchor matching, with no compile error and no other
//! failing test. These tests assert the actual behavior the two features
//! grant, so losing the features fails a test regardless of how they were
//! lost.

#[test]
fn serde_json_preserves_insertion_order() {
	let v: serde_json::Value = serde_json::from_str(r#"{"z":1,"a":2,"m":3}"#).unwrap();
	assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"z":1,"a":2,"m":3}"#);
}

#[test]
fn serde_json_keeps_arbitrary_precision_literals() {
	let v: serde_json::Value = serde_json::from_str(r#"{"x":1.0,"y":12345678901234567}"#).unwrap();
	assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"x":1.0,"y":12345678901234567}"#);
}
