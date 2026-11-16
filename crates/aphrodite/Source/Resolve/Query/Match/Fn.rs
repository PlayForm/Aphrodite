//! The line matcher both query filters share.

use super::super::Fold;

/// The lines of `Content` that contain `Needle`, in their original order.
///
/// Borrowed lines, not a rendered string: the two callers answer a miss differently (the
/// endpoint says how many lines it searched, the resolver hands the content back behind a
/// notice), so the shared part stops at the decision and each caller renders its own
/// answer. Returning `Vec<&str>` is also what keeps the pass-through case from allocating a
/// copy of the content.
///
/// `Needle` must already be lowercased - the callers cap and fold their query before
/// they get here, and doing it twice would either double the cost or silently change which
/// text is compared.
pub(crate) fn Fn<'a>(Content:&'a str, Needle:&str, Fold:&Fold::Enum::Enum) -> Vec<&'a str> {
	Content
		.lines()
		.filter(|Line| {
			let Lower = match Fold {
				Fold::Enum::Enum::ASCII => Line.to_ascii_lowercase(),
				Fold::Enum::Enum::Unicode => Line.to_lowercase(),
			};

			Lower.contains(Needle)
		})
		.collect()
}
