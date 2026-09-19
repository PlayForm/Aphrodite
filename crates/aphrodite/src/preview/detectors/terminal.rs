//! terminal shape detector (Phase 6: exit-code / shell-trace rules).
//!
//! Conservative on purpose: the terminal arm must NOT become a universal
//! detector (every `Error:` line would classify terminal). Only explicit
//! shell-trace signals count - an `exit code:` line or `$ `-prompt / `exit
//! code` majority.

use crate::preview::input::Input;

/// Shell/exit-code trace signals: an explicit `exit code:` line, a majority
/// of `$ `-prompt lines, or a `command not found` / `zsh:` / `bash:` error
/// trace.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let has_exit_code = inp.raw.lines().any(|l| l.contains("exit code:"));
	let prompt_majority = inp.majority(|l| l.trim_start().starts_with("$ "), 1);
	let shell_err = inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with("zsh:") || t.starts_with("bash:") || t.contains("command not found")
	});
	has_exit_code || prompt_majority || shell_err
}