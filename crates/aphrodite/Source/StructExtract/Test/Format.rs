//! Byte-safe truncation: a multi-byte character must never panic a byte-offset cut,
//! and a signature must still fit the line cap after the cut.

use crate::StructExtract::{
	Format::{Cap::Fn::Fn as Cap, Floor::Fn as FloorBoundary, Parameters::Fn as Params, Signature::Fn as Sig},
	Maximum::{Parameters::Const as Parameters, Signature::Const as Signature},
};

#[test]
fn TestFloorBoundaryNeverPanicsOnMultibyte() {
	assert_eq!(FloorBoundary("hello", 10), "hello");
	let s = "é".repeat(200); // every char is 2 bytes; any odd offset is mid-char
	let Out = FloorBoundary(&s, 99);
	assert!(Out.len() <= 99);
	assert!(s.is_char_boundary(Out.len()));
}

#[test]
fn TestSigMultibyteSignatureDoesNotPanic() {
	let name = "é".repeat(40);
	// Must not panic when truncating a signature full of multi-byte chars.
	let s = Sig("fn", &name);
	assert!(s.len() <= Signature::Const + 3); // +3 for "..."
}

/// The caps' boundary: a value exactly at the cap comes back whole.
///
/// The previous `checked_sub` test read equality as an overrun, so a parameter list at exactly
/// `Maximum::Parameters::Const` bytes lost its last three characters to `...`, and a signature at exactly
/// `Maximum::Signature::Const` did the same - the same size, three characters less information.
#[test]
fn TestCapsFitsUnchangedAtBoundary() {
	let ExactParams = "a".repeat(Parameters::Const);
	let OverParams = "a".repeat(Parameters::Const + 1);

	assert_eq!(Params(&ExactParams), ExactParams, "a list at the cap must not be truncated");
	assert_eq!(
		Params(&OverParams).len(),
		Parameters::Const,
		"a list over the cap keeps the cap's width"
	);
	assert!(Params(&OverParams).ends_with("..."));

	let ExactName = "n".repeat(Signature::Const - "fn ".len());
	let ExactLine = format!("fn {ExactName}");

	assert_eq!(ExactLine.len(), Signature::Const);
	assert_eq!(Sig("fn", &ExactName), ExactLine, "a signature at the cap must not be truncated");
	assert_eq!(Cap(ExactLine.clone()), ExactLine, "the line cap must not truncate at equality");
}
