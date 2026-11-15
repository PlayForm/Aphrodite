//! The `exit code: N` line fallback for a poll result that was never wrapped.

/// The last `exit code: N` in the output, parsed.
pub(crate) fn Fn(Output:&str) -> Option<i32> {
	Output.lines().rev().find_map(|Line| {
		Line.split("exit code:")
			.nth(1)
			.and_then(|Rest| Rest.split_whitespace().next())
			.and_then(|Number| Number.parse::<i32>().ok())
	})
}
