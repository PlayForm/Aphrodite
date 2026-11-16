//! The identity of a command line.

/// The CCR hash of a command line - the poll worker's deduplication key.
///
/// It identifies the *command*, not a run of it: two simultaneous runs of one
/// command share this value on purpose, which is why it cannot stand in for an
/// execution identity. All three identities used to be `String` and were therefore
/// interchangeable; this type is what makes storing one in another's slot a compile
/// error rather than a silent mis-attribution.
///
/// Constructible from text because a command identity is minted from a command line
/// and from nowhere else, and it hands its text back through `as_str` rather than
/// `Deref` - a `&str` parameter must not be able to absorb a `CommandID` by
/// coercion, or the boundary the type exists to draw would have a hole in it.
///
/// ```compile_fail
/// use aphrodite::State::Identity::{Command::Struct::CommandID, Execution::Struct::ExecutionID};
///
/// let command: CommandID = "cargo build".into();
/// let execution: ExecutionID = command;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandID(String);

impl CommandID {
	/// The identity's text.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl From<String> for CommandID {
	fn from(Value:String) -> Self { Self(Value) }
}

impl From<&str> for CommandID {
	fn from(Value:&str) -> Self { Self(Value.to_string()) }
}

impl std::fmt::Display for CommandID {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { Formatter.write_str(&self.0) }
}
