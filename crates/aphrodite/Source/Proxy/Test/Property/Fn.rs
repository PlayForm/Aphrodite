//! Property test: the classifier and the metadata generator never panic on
//! arbitrary UTF-8, and the metadata respects its own documented invariants.

// ── T8: property tests ───────────────────────────────────────
use proptest::{prop_assert, proptest};

use crate::Proxy::{
	Analysis::Fn::Fn as AnalyseContent,
	Metadata::Fn::Fn as GenerateMetadata,
	Test::Support::Analysis::Fn::Fn as NamedAnalysis,
};

proptest! {
	/// `Analysis::Classify` and `Metadata::Fn` must never panic on arbitrary
	/// UTF-8 input (this is literally the module's threat model - arbitrary tool
	/// output), and the generated metadata must respect its own documented
	/// invariants: no pipe/newline, <=400 chars.
	#[test]
	fn PropClassifierAndMetadataNeverPanic(s in ".*") {
		let Detected = AnalyseContent(&s);
		let Named = NamedAnalysis(&s, Detected.Ct);

		let Metas:Vec<String> = [&Detected, &Named].into_iter().map(GenerateMetadata).collect();
		let Violations:Vec<&String> = Metas
			.iter()
			.filter(|meta| meta.contains('|') || meta.contains('\n') || meta.chars().count() > 400)
			.collect();

		prop_assert!(Violations.is_empty(), "metadata broke its own invariants: {:?}", Violations);
	}
}
