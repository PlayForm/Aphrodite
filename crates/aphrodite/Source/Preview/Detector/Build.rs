//! Build-output shape detector.
//!
//! Build output is recognised by its verbs - `Compiling`, `Finished` - and by
//! diagnostic lines. This detector votes on those markers. It is not a compiler
//! detector: a single `error:` line is not enough, because a terminal trace of a
//! failed command also carries one and belongs on the terminal arm.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like compiler/build output.
///
/// Cargo/rustc-style verb lines (`Compiling`, `Finished`, `error[`, `error:`,
/// `warning:`, `-->`) count as markers; two markers, or one verb plus a
/// diagnostic line, are enough. It is not a success check: the detector answers
/// "is this build output", never "did the build pass".
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let BuildVerbs = Inp
		.NonEmpty
		.iter()
		.filter(|l| {
			let t = l.trim_start();
			["Compiling ", "Building ", "Finished ", "Checking ", "Linking ", "--> "]
				.iter()
				.any(|p| t.starts_with(p))
		})
		.count();
	let BuildErrs = Inp
		.NonEmpty
		.iter()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("error[")
				|| t.starts_with("error:")
				|| t.starts_with("Error:")
				|| t.starts_with("warning[")
				|| t.starts_with("warning:")
				|| t.starts_with("Warning:")
		})
		.count();
	BuildVerbs + BuildErrs >= 2
}
