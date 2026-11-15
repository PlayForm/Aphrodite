//! One tool event, observed before any decision is taken about it.

use crate::Hook::{Observation::Outcome::Enum::Enum as Outcome, Path::Resolved::Struct::Struct as ResolvedPath};

/// What one tool or terminal call did, as a value.
///
/// Telemetry used to be recorded inline, ahead of each gate, so the recording
/// was only as reliable as the next edit to the gate sequence: a new early
/// return silently dropped a call's event. Holding the observation in a value
/// and applying it before plan selection makes that impossible - the pipeline
/// observes once, then decides, and the decision cannot reach the model without
/// the observation having been applied first.
///
/// `WrotePath` and `ReferencedPath` are distinct fields because they feed
/// different signals: a written path is what the call changed (the `WrotePath`
/// telemetry column), a referenced path is what it looked at (the
/// `ReferencedFiles` catalog). A `patch` both writes and references; a
/// `read_file` only references.
pub struct Struct {
	/// Tool name, or `"terminal"` for terminal output.
	pub Tool:String,
	/// Whether the call reported failure.
	pub Outcome:Outcome,
	/// FNV-1a of the tool plus its normalized arguments (similarity key).
	pub ArgsSig:u64,
	/// FNV-1a of the failure's type and first message line, when it failed.
	pub ErrorSig:Option<u64>,
	/// Byte length of the call's result content.
	pub Bytes:usize,
	/// Path this call wrote, when it wrote one.
	pub WrotePath:Option<ResolvedPath>,
	/// Path this call read or searched, when one was named.
	pub ReferencedPath:Option<ResolvedPath>,
}
