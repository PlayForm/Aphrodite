use super::AphroditeState;

/// One recorded tool/terminal call (P2/T6). Only hashes of args/errors are
/// stored, never raw args, so no PII lands in state.
#[derive(Debug, Clone)]
pub struct ToolEvent {
	/// Turn on which the call happened.
	pub turn:usize,
	/// Tool name (or `"terminal"` for terminal output).
	pub tool:String,
	/// FNV-1a of tool + normalized args (P8 similarity key).
	pub sig:u64,
	/// `status != "error" && returncode == 0` (fail-open: missing → true).
	pub ok:bool,
	/// FNV-1a of `error_type` + first line of `error_message`, when failing.
	pub error_sig:Option<u64>,
	/// Byte length of the call's result content.
	pub bytes:usize,
	/// `write_file`/`patch` target path, when this call wrote a file (P11).
	pub wrote_path:Option<String>,
}

impl AphroditeState {
	/// Record a per-call tool event into the bounded ring (P2/T6). Caps at 200
	/// entries, evicting the front (oldest) - same eviction style as
	/// `recent_markers`.
	pub fn record_tool_event(&mut self, event:ToolEvent) {
		self.tool_events.push_back(event);
		while self.tool_events.len() > 200 {
			self.tool_events.pop_front();
		}
	}
}
