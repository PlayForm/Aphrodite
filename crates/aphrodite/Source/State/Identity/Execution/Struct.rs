//! The identity of one process execution.

/// The process identity a poll result names (`session_id` on the wire).
///
/// It is learned, not minted: the first poll attributed to a task records the
/// execution identity it names, and every later poll naming the same process
/// reaches the same task through it. That is why it cannot stand in for a command
/// identity - a command identity is known before the process exists, an execution
/// identity only after something reports it.
///
/// ```compile_fail
/// use aphrodite::State::Identity::{Command::Struct::CommandID, Execution::Struct::ExecutionID};
///
/// let execution: ExecutionID = "proc-7".into();
/// let command: CommandID = execution;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutionID(String);

impl ExecutionID {
	/// The identity's text.
	pub fn as_str(&self) -> &str { &self.0 }
}

impl From<String> for ExecutionID {
	fn from(Value:String) -> Self { Self(Value) }
}

impl From<&str> for ExecutionID {
	fn from(Value:&str) -> Self { Self(Value.to_string()) }
}

impl std::fmt::Display for ExecutionID {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { Formatter.write_str(&self.0) }
}
