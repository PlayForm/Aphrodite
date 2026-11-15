//! `Extract::Exit::Fn`: the `EXIT:<digits>` trace.

/// First `EXIT:<digits>` marker in the content (the house shell-trace
/// convention `echo "EXIT:$?"`); `"?"` when absent.
///
/// The digits are read greedily but only digits: a line such as
/// `EXIT:0 (ok)` must yield `0`, not `0 (ok)`. Anything after the number is
/// prose, and a preview that carried prose in `{exit}` would break every
/// template that compares it.
pub(crate) fn Fn(Lines:&[&str]) -> String {
	Lines
		.iter()
		.find_map(|l| {
			let Rest = l.split_once("EXIT:").map(|(_, Rest)| Rest)?;
			let Code:String = Rest.chars().take_while(|c| c.is_ascii_digit()).collect();
			match Code.is_empty() {
				true => None,
				false => Some(Code),
			}
		})
		.unwrap_or_else(|| "?".to_string())
}
