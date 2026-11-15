//! Merge a segment's error hint into its preview brackets.

use std::borrow::Cow;

use crate::ChainSplit::ErrorHint::Fn::Fn as SegmentErrorHint;

/// Decorate a preview with the segment's own error signal.
///
/// Borrowed when the segment carries no hint, owned when it does - the
/// decoration is an optional transform, so a clean segment allocates nothing
/// here. The hint is merged INSIDE the preview's brackets
/// (`[text:2L 63B | scanning... ⚠ grep: ...]`), never appended after the `]`:
/// a trailing hint would read as marker syntax to the parser.
///
/// A build-type preview already carries the first `error[...]` message, so the
/// hint is skipped there rather than duplicated.
///
/// The caller's `.into_owned()` is deliberate and is the one place this borrow
/// cannot be carried: the decorated string is stored in the prepared segment,
/// which outlives the preview it decorates, so ownership has to be taken at
/// this boundary. The saving the `Cow` buys is the clean-segment case, where
/// nothing is allocated at all.
pub(crate) fn Fn<'a>(Preview:&'a str, Text:&str) -> Cow<'a, str> {
	let Hint = (!Preview.contains("error[")).then(|| SegmentErrorHint(Text)).flatten();

	Hint.map_or(Cow::Borrowed(Preview), |Hint| {
		let Inner = Preview.trim_end_matches(']');

		Cow::Owned(format!("{Inner} ⚠ {Hint}]"))
	})
}
