//! Rows from the path-grouped `matches_text` block.

use serde_json::{Map, Value};

/// Rebuild `matches_text` as the same grep-style rows as `Rows::Array`.
///
/// The block is a `path` header line followed by indented `<line>: <content>`
/// rows. Only an indented row under a known path becomes a row, a header resets
/// the current path, and a blank line leaves it alone - so a malformed block
/// produces fewer rows rather than wrong ones.
///
/// `None` means "this block produced no rows", which is what the caller needs: a
/// block that is absent and a block with nothing parseable are both "there is no
/// per-match content here", and both fall through to the count-only payload.
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<Vec<String>> {
	let Text = Object.get("matches_text").and_then(Value::as_str)?;

	let Rows = Text
		.lines()
		.fold((None::<&str>, Vec::new()), |(Path, mut Rows), Raw| {
			let Trimmed = Raw.trim();
			let Indented = Raw.trim_start().len() != Raw.len();

			match (Trimmed.is_empty(), Indented) {
				// Blank line: no row, the path carries over.
				(true, _) => (Path, Rows),
				// Indented row under the current path: "<line>: <content>".
				(false, true) => {
					match (Path, Trimmed.split_once(':')) {
						(Some(Path), Some((Number, Rest))) => {
							Rows.push(format!("{}:{}:{}", Path, Number.trim(), Rest.trim_start()));

							(Some(Path), Rows)
						},
						_ => (Path, Rows),
					}
				},
				// A path header for the rows that follow.
				(false, false) => (Some(Trimmed), Rows),
			}
		})
		.1;

	(!Rows.is_empty()).then_some(Rows)
}
