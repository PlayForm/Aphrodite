//! The accumulator's bound at a size a test can count.
//!
//! `Update/Cap.rs` proves the shipped 256 KB bound holds under real polls; this file
//! proves the ARITHMETIC, on an accumulator whose bound is ten bytes, so every byte the
//! separator and the truncation charge is visible in the assertion instead of being
//! inside a quarter megabyte.

use crate::PollWorker::Output::{
	Append::{Enum::Append, Fn::Fn as AppendTo},
	Struct::Accumulator,
};

#[test]
fn Separator() {
	let mut Acc = Accumulator::NewWithin(10);

	assert_eq!(AppendTo(&mut Acc, "abcdef"), Append::Complete);
	assert_eq!(Acc.AsStr(), "abcdef");

	// Room is 10 - 6 = 4; the newline takes one, so three characters of "ghij" fit.
	assert_eq!(AppendTo(&mut Acc, "ghij"), Append::Truncated);
	assert_eq!(Acc.AsStr(), "abcdef\nghi", "the newline is part of the buffer the model reads");
	assert_eq!(Acc.Len(), 10, "the bound holds exactly");
}

#[test]
fn Full() {
	let mut Acc = Accumulator::NewWithin(4);
	assert_eq!(AppendTo(&mut Acc, "abcd"), Append::Complete);

	// No room for the separator, so no room for any content: a trailing newline would
	// be a byte spent to say nothing.
	assert_eq!(AppendTo(&mut Acc, "e"), Append::Full);
	assert_eq!(Acc.AsStr(), "abcd");
	assert_eq!(Acc.Len(), 4);
}

#[test]
fn Utf8() {
	let mut Acc = Accumulator::NewWithin(5);

	// Six bytes of three two-byte characters: five would split the third.
	assert_eq!(AppendTo(&mut Acc, "ééé"), Append::Truncated);
	assert_eq!(Acc.AsStr(), "éé");
	assert_eq!(Acc.Len(), 4, "the kept prefix stops at the last whole character");
	assert!(Acc.AsStr().chars().all(|Char| Char == 'é'));
}

#[test]
fn First() {
	let mut Acc = Accumulator::NewWithin(3);

	assert_eq!(AppendTo(&mut Acc, "abc"), Append::Complete);
	assert_eq!(Acc.AsStr(), "abc", "an empty buffer owes no separator");
}

#[test]
fn Empty() {
	let mut Acc = Accumulator::NewWithin(10);
	AppendTo(&mut Acc, "abc");

	assert_eq!(Acc.Take(), "abc");
	assert_eq!(Acc.Len(), 0);
	// The bound is the accumulator's, not the buffer's: it survives the move-out.
	assert_eq!(AppendTo(&mut Acc, "0123456789ab"), Append::Truncated);
	assert_eq!(Acc.Len(), 10);
}
