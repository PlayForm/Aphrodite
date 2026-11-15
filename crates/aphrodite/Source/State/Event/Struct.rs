//! One recorded tool/terminal call.

/// One recorded tool/terminal call (P2/T6). Only hashes of args/errors are
/// stored, never raw args, so no PII lands in state.
#[derive(Debug, Clone)]
pub struct ToolEvent {
	/// Turn on which the call happened.
	pub Turn:usize,
	/// Tool name (or `"terminal"` for terminal output).
	pub Tool:String,
	/// FNV-1a of tool + normalized args (P8 similarity key).
	pub Sig:u64,
	/// `status != "error" && returncode == 0` (fail-open: missing → true).
	pub Okay:bool,
	/// FNV-1a of `ErrorType` + first line of `ErrorMessage`, when failing.
	pub ErrorSig:Option<u64>,
	/// Byte length of the call's result content.
	pub bytes:usize,
	/// `write_file`/`patch` target path, when this call wrote a file (P11).
	pub WrotePath:Option<String>,
}
