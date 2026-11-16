//! The code reducer.
//!
//! Source files are recognised by their declaration lines, so the reduction is a scan for the nine
//! declaration shapes Aphrodite cares about across Rust, Python, Go and JS/TS. Each match becomes one
//! `[kind] line` entry, bounded so a generated file cannot produce a "reduction" bigger than its input.
//!
//! The lowercase form of each line is computed ONCE per line and carried with it in a `Normalised`
//! value, which is why the matcher takes one argument instead of a line and its lowercase form as two
//! loosely-related strings.

use crate::Stage2::Reduce::{
	Code::{Matcher::Fn::Fn as Matcher, Normalised::Struct::Struct as Normalised},
	Sample::Struct::Struct as Sample,
};

/// Signatures the reduction shows: fifty.
const SignaturesShown:usize = 50;

/// Characters of one signature line: one hundred and twenty.
const SignatureShown:usize = 120;

/// Reduce code: extract function/struct/class signatures.
///
/// The total number of recognised declarations is exact even though only the first fifty are retained,
/// so a file with three hundred declarations still costs fifty lines of memory. It is not a parser: a
/// declaration split across two lines is matched on whichever half carries the prefix.
pub fn ReduceCode(Content:&str) -> Option<String> {
	let mut Signatures = Sample::<String, SignaturesShown>::new();

	Content.lines().for_each(|Line| {
		let Normalised = Normalised::new(Line);

		Matcher(&Normalised).into_iter().for_each(|Kind| {
			Signatures.Push(|| {
				format!(
					"  [{}] {}",
					Kind.Kind(),
					Normalised.Trimmed().chars().take(SignatureShown).collect::<String>()
				)
			});
		});
	});

	// `None` for "no declarations at all", which is the same answer the caller got before; the body is
	// built only when there is something to build, so a file of prose allocates nothing.
	(!Signatures.IsEmpty()).then(|| {
		let mut Result = String::from("[code structure]");

		Signatures.IntoSample().into_iter().for_each(|Signature| {
			Result.push('\n');
			Result.push_str(&Signature);
		});

		Result
	})
}
