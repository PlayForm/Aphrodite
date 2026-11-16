//! The identity of stored content.

/// The CCR hash of a stored payload - what a marker resolves and what the inline
/// store is keyed by.
///
/// Distinct from the command identity on purpose, and the distinction is not
/// cosmetic: a poll worker's marker carries the *command* hash before any output
/// exists, and the *content* hash only once the output has been stored. Two runs of
/// one command share a command identity and never share a content identity, so
/// confusing the two would hand the model a marker that resolves to another run's
/// output.
///
/// ```compile_fail
/// use aphrodite::State::Identity::{Command::Struct::CommandID, Content::Struct::ContentHash};
///
/// fn store(hash: ContentHash) -> usize { hash.as_str().len() }
/// let command: CommandID = "cargo build".into();
/// let size = store(command);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(String);

impl ContentHash {
	/// The identity's text.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl From<String> for ContentHash {
	fn from(Value:String) -> Self { Self(Value) }
}

impl From<&str> for ContentHash {
	fn from(Value:&str) -> Self { Self(Value.to_string()) }
}

impl std::fmt::Display for ContentHash {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { Formatter.write_str(&self.0) }
}
