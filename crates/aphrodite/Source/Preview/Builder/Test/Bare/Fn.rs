//! A bare `test <name> ... ok|FAILED|ignored` log.

use crate::Preview::Input::Struct::Struct as Input;
use super::super::Totals::Struct::Struct as Totals;

/// The result markers a bare log prints, in the order they are tested.
///
/// The order is the contract: a line carrying both `... ok` and `... FAILED` counts as a
/// pass, because that is what the previous reader's guard chain did.
const Markers:[&str; 3] = ["... ok", "... FAILED", "... ignored"];

/// Count the per-test result lines of a log with no summary block.
///
/// This is the last resort: a run whose summary shape nothing recognised still reports an
/// honest pass/fail tally from its own result lines, instead of previewing as a bare line
/// count. It is not a substitute for a summary: a log that carries both uses the summary.
pub(crate) fn Fn(Inp:&Input<'_>) -> Option<Totals> {
	let Kinds:Vec<usize> = Inp
		.raw
		.lines()
		.filter_map(|Line| {
			let Rest = Line.trim().strip_prefix("test ")?;

			Markers.into_iter().position(|Marker| Rest.contains(Marker))
		})
		.collect();

	(!Kinds.is_empty()).then(|| {
		Totals {
			Pass:Kinds.iter().filter(|Kind| **Kind == 0).count(),
			Fail:Kinds.iter().filter(|Kind| **Kind == 1).count(),
			Ignored:Kinds.iter().filter(|Kind| **Kind == 2).count(),
		}
	})
}
