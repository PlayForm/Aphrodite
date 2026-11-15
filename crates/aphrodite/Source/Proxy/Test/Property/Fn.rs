//! Property test: the classifier and the metadata generator never panic on
//! arbitrary UTF-8, and the metadata respects its own documented invariants.

// ── T8: property tests ───────────────────────────────────────
use proptest::{prop_assert, proptest};

use crate::Proxy::{Detect::Fn::Fn as ProxyDetectContentType, Metadata::Fn::Fn as GenerateMetadata};

proptest! {
	/// `detect_content_type` and `GenerateMetadata` must never panic on
	/// arbitrary UTF-8 input (this is literally the module's threat
	/// model - arbitrary tool output), and the generated metadata must
	/// respect its own documented invariants: no pipe/newline, <=400 chars.
	#[test]
	fn PropClassifierAndMetadataNeverPanic(s in ".*") {
		let Ct = ProxyDetectContentType(&s);
		let meta = GenerateMetadata(&s, Ct);
		prop_assert!(!meta.contains('|'));
		prop_assert!(!meta.contains('\n'));
		prop_assert!(meta.chars().count() <= 400);
	}
}

// ── 04-T8: SSE detection + handler-level streaming tests ──────
