//! The identities round-trip their text and keep their own type.
//!
//! The compile-time half of this contract is the `compile_fail` fixture on each
//! type; what a test can still add is that the constructors preserve the bytes they
//! were given - a hash that loses a character is unresolvable, and the marker's
//! hash is the model's only handle on the content.

use crate::State::Identity::{
	Command::Struct::CommandID,
	Content::Struct::ContentHash,
	Execution::Struct::ExecutionID,
};

#[test]
fn TestIdentitiesRoundTripTheirText() {
	let Command = CommandID::from("a3f1c9");
	let Execution = ExecutionID::from(String::from("proc-7"));
	let Content = ContentHash::from("b7d2e0");

	assert_eq!(Command.as_str(), "a3f1c9");
	assert_eq!(Execution.as_str(), "proc-7");
	assert_eq!(Content.as_str(), "b7d2e0");
}

#[test]
fn TestIdentityDisplayIsTheBareText() {
	// The tracing fields render the identity through `Display`, so the log line
	// must carry the hash and nothing around it.
	assert_eq!(CommandID::from("abc").to_string(), "abc");
	assert_eq!(ContentHash::from("abc").to_string(), "abc");
}

#[test]
fn TestIdentityEqualityIsExact() {
	// Identity equality is textual equality within one category: two command
	// identities with the same text are the same command. Across categories the
	// comparison does not exist at all - that is the `compile_fail` fixture's job,
	// because a test can only assert what compiles.
	assert_eq!(CommandID::from("same"), CommandID::from("same"));
	assert_ne!(CommandID::from("same"), CommandID::from("other"));
	assert_eq!(ExecutionID::from("same"), ExecutionID::from(String::from("same")));
}
