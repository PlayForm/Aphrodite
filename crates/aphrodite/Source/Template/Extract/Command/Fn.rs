//! The command that produced the output.

/// First command-looking line (trimmed start `$` or `>`), stripped of its leading
/// `$`/`>` and whitespace, truncated to 200 chars; `None` when absent.
///
/// The prefix is what makes this safe: a bare line of shell output is
/// indistinguishable from a command, so only an explicitly prompted line qualifies.
/// A wrong `{cmd}` would render as if the agent had run something it never ran,
/// which is worse than an empty string.
///
/// `None` rather than `""`: "no command line" is absence, and the renderer decides
/// how an absent variable reads.
pub(crate) fn Fn(Lines:&[&str]) -> Option<String> {
	Lines.iter().find_map(|Line| {
		let Trimmed = Line.trim_start();
		let Rest = Trimmed.strip_prefix('$').or_else(|| Trimmed.strip_prefix('>'))?;
		let Command = Rest.trim();

		(!Command.is_empty()).then(|| Command.chars().take(200).collect::<String>())
	})
}
