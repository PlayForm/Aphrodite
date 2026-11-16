//! The identities round-trip their text and keep their own type.
//!
//! The compile-time half of this contract is the `compile_fail` fixture on each
//! type; what a test can still add is that the constructors preserve the bytes they
//! were given - a hash that loses a character is unresolvable, and the marker's
//! hash is the model's only handle on the content.

use crate::State::Identity::{
	Command::Struct::Struct as Command,
	Content::Struct::Struct as Digest,
	Execution::Struct::Struct as Execution,
};

#[test]
fn RoundTrip() {
	let Command = Command::from("a3f1c9");
	let Execution = Execution::from(String::from("proc-7"));
	let Content = Digest::from("b7d2e0");

	assert_eq!(Command.as_str(), "a3f1c9");
	assert_eq!(Execution.as_str(), "proc-7");
	assert_eq!(Content.as_str(), "b7d2e0");
}

#[test]
fn Display() {
	// The tracing fields render the identity through `Display`, so the log line
	// must carry the hash and nothing around it.
	assert_eq!(Command::from("abc").to_string(), "abc");
	assert_eq!(Digest::from("abc").to_string(), "abc");
}

#[test]
fn Equality() {
	// Identity equality is textual equality within one category: two command
	// identities with the same text are the same command. Across categories the
	// comparison does not exist at all - that is the `compile_fail` fixture's job,
	// because a test can only assert what compiles.
	assert_eq!(Command::from("same"), Command::from("same"));
	assert_ne!(Command::from("same"), Command::from("other"));
	assert_eq!(Execution::from("same"), Execution::from(String::from("same")));
}
