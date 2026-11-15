//! The JS/TS extractor: declarations, real arrow-function bindings, and the
//! trailing-comment `=>` that must never be mistaken for one.

use crate::StructExtract::Extract::Fn as ExtractCodeStructure;

/// `const x = 5; // map => y` must not be recorded as an arrow function -
/// the `=>` is inside a trailing comment, not part of the binding's RHS.
#[test]
fn TestExtractJSArrowFnGuardIgnoresCommentOnlyArrow() {
	let Code = "const x = 5; // map => y\n";
	let r = ExtractCodeStructure(Code, "js");
	match r.get("fns") {
		Some(Fns) => {
			assert!(
				!Fns.iter().any(|s| s.contains('x')),
				"trailing-comment `=>` must not be mistaken for an arrow fn: {Fns:?}"
			);
		},
		None => {},
	}
}

/// A real arrow function assignment must still be detected.
#[test]
fn TestExtractJSArrowFnStillDetected() {
	let Code = "const add = (a, b) => a + b;\n";
	let r = ExtractCodeStructure(Code, "js");
	let Fns = &r["fns"];
	assert!(
		Fns.iter().any(|s| s.contains("add")),
		"real arrow fn should still be extracted: {Fns:?}"
	);
}
