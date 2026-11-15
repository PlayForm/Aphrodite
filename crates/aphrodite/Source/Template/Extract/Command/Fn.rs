//! `Extract::Command::Fn`: the command that produced the output.

/// First command-looking line (trimmed start `$` or `>`), stripped of its
/// leading `$`/`>` and whitespace, truncated to 200 chars; `""` when absent.
///
/// The prefix is what makes this safe: a bare line of shell output is
/// indistinguishable from a command, so only an explicitly prompted line
/// qualifies. A wrong `{cmd}` would render as if the agent had run something it
/// never ran, which is worse than an empty string.
pub(crate) fn Fn(Lines:&[&str]) -> String {
	for l in Lines {
		let t = l.trim_start();
		if let Some(Rest) = t.strip_prefix('$').or_else(|| t.strip_prefix('>')) {
			let Command = Rest.trim();
			if !Command.is_empty() {
				return Command.chars().take(200).collect();
			}
		}
	}
	String::new()
}
