//! Split marked tool output back into per-segment pieces.

use crate::ChainSplit::Const::SegMarker;

/// Split tool output on segment markers into (segment_index, text) pairs.
/// Output before the first marker belongs to segment 0. Marker lines are
/// removed; surrounding blank lines are trimmed per segment.
pub fn Fn(Output:&str) -> Vec<(usize, String)> {
	let mut Parts:Vec<(usize, String)> = Vec::new();
	let mut Current = String::new();
	let mut CurrentIndex = 0usize;

	Output.lines().for_each(|Line| {
		let Trimmed = Line.trim();

		match Trimmed == SegMarker {
			true => {
				match Current.trim().is_empty() {
					true => (),
					false => {
						Parts.push((CurrentIndex, Current.trim().to_string()));
					},
				}

				CurrentIndex += 1;

				// `clear()` rather than `cur = String::new()`: same observable
				// behaviour, but the buffer is reused - clippy's
				// `new_instead_of_clear` denies the reallocation under
				// `-D warnings` on the CI toolchain.
				Current.clear();
			},
			false => {
				Current.push_str(Line);
				Current.push('\n');
			},
		}
	});

	match Current.trim().is_empty() {
		true => (),
		false => {
			Parts.push((CurrentIndex, Current.trim().to_string()));
		},
	}

	Parts
}
