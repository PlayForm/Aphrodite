//! The `EXIT:<digits>` trace.

/// First `EXIT:<digits>` marker in the content (the house shell-trace convention
/// `echo "EXIT:$?"`), or `None` when the content carries no trace.
///
/// The digits are read greedily but only digits: a line such as `EXIT:0 (ok)` must
/// yield `0`, not `0 (ok)`. Anything after the number is prose, and a preview that
/// carried prose in `{exit}` would break every template that compares it.
///
/// `None` rather than `"?"`: the placeholder is a rendering decision, and the one
/// place that renders this variable applies it. An extractor answering `"?"` made
/// "no trace" indistinguishable from a trace whose digits were literally `?`.
pub(crate) fn Fn(Lines:&[&str]) -> Option<String> {
	Lines.iter().find_map(|Line| {
		let Rest = Line.split_once("EXIT:").map(|(_Prefix, Rest)| Rest)?;
		let Code:String = Rest.chars().take_while(|Char| Char.is_ascii_digit()).collect();

		(!Code.is_empty()).then_some(Code)
	})
}
