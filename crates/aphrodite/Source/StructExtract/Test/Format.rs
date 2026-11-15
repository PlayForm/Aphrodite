//! Byte-safe truncation: a multi-byte character must never panic a byte-offset cut,
//! and a signature must still fit the line cap after the cut.

use crate::StructExtract::{
	Format::{Floor::Fn as FloorBoundary, Signature::Fn as Sig},
	MaximumSignatureLength::Const as MaximumSigLength,
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
	assert!(s.len() <= MaximumSigLength + 3); // +3 for "..."
}
