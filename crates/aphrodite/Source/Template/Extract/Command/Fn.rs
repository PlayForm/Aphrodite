//! `Extract::Command::Fn`: the command that produced the output.

/// First command-looking line (trimmed start `$` or `>`), stripped of its
/// leading `$`/`>` and whitespace, truncated to 200 chars; `""` when absent.
///
/// The prefix is what makes this safe: a bare line of shell output is
/// indistinguishable from a command, so only an explicitly prompted line
/// qualifies. A wrong `{cmd}` would render as if the agent had run something it
/// never ran, which is worse than an empty string.
pub(crate) fn Fn(Lines:&[&str]) -> String {
	Lines
		.iter()
		.find_map(|l| {
			let t = l.trim_start();
			let Rest = t.strip_prefix('$').or_else(|| t.strip_prefix('>'))?;
			let Command = Rest.trim();
			match Command.is_empty() {
				true => None,
				false => Some(Command.chars().take(200).collect::<String>()),
			}
		})
		.unwrap_or_default()
}
