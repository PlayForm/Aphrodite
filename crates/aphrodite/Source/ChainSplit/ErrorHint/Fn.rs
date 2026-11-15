//! The per-segment error hint.

/// Extract a compact error hint from a segment's output, when it carries an
/// error signal (exit code, error:/Error:, FAILED/failed, panic, Traceback,
/// etc.). Returns the first matching line, trimmed to ≤60 chars.
///
/// Tier 3: per-segment error hints. This is CONTENT-level vocabulary (what
/// the segment itself says), never mechanism vocabulary - the invisibility
/// contract limits hints to the segment's own output, so the LLM learns
/// "that segment failed" from the hint without any chain/split wording.
pub fn Fn(Content:&str) -> Option<String> {
	let Candidates:Vec<&str> = Content
		.lines()
		.filter_map(|Line| {
			let l = Line.trim();

			match l.is_empty() {
				true => None,
				false => {
					let Lower = l.to_lowercase();
					let IsError = l.contains("exit code:")
						|| Lower.contains("error")
						|| Lower.contains("failed")
						|| Lower.contains("failure")
						|| Lower.contains("panic")
						|| Lower.contains("traceback")
						|| Lower.contains("fatal")
						|| Lower.contains("cannot")
						|| Lower.contains("not found")
						|| Lower.contains("no such file")
						|| Lower.contains("permission denied")
						|| Lower.contains("unresolved import")
						|| Lower.contains("undefined");

					match IsError {
						true => Some(l),
						false => None,
					}
				},
			}
		})
		.collect();

	// `error[EXXXX]: msg` is the most informative shape - it wins over the
	// first plain signal.
	match Candidates.iter().find(|l| l.contains("error[")) {
		Some(l) => Some(l.chars().take(60).collect()),
		None => Candidates.first().map(|l| l.chars().take(60).collect()),
	}
}
