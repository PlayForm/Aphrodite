//! build-output shape detector.

use crate::preview::input::Input;

/// cargo/rustc-style verb lines (`Compiling`, `Finished`, `error[`, `error:`,
/// `warning:`, `-->`) - >=2 markers, or a verb plus an error/warning line.
/// A lone `error: broke` terminal trace does NOT count (stays on the
/// terminal arm).
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let build_verbs = inp
		.non_empty
		.iter()
		.filter(|l| {
			let t = l.trim_start();
			["Compiling ", "Building ", "Finished ", "Checking ", "Linking ", "--> "]
				.iter()
				.any(|p| t.starts_with(p))
		})
		.count();
	let build_errs = inp
		.non_empty
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
	build_verbs + build_errs >= 2
}
