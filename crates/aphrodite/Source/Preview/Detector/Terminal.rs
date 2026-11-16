//! Terminal shape detector.
//!
//! The terminal arm must not become a universal detector - every `Error:` line
//! would classify as terminal - so only explicit shell-trace signals count here.
//! It is not a fallback: content that matches nothing else lands on the generic
//! arm, not on this one.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like a shell/terminal trace.
///
/// An explicit `exit code:` line, a majority of `$ `-prompt lines, or a
/// `command not found` / `zsh:` / `bash:` error trace. It is not an
/// error-message detector: a lone `error: ...` line deliberately does not count.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let Exit = Inp.raw.lines().any(|l| l.contains("exit code:"));
	let PromptMajority = Inp.Majority(|l| l.trim_start().starts_with("$ "), 1);
	let ShellError = Inp.raw.lines().any(|l| {
		let t = l.trim();
		t.starts_with("zsh:") || t.starts_with("bash:") || t.contains("command not found")
	});
	Exit || PromptMajority || ShellError
}
