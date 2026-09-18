//! chain_split - fine-grained decomposition of chained shell commands.
//!
//! LLMs habitually chain commands (`cd x && cargo build && cargo test`) into
//! one terminal call. The resulting single output blob defeats per-type
//! compression: it is one `build`-typed entry, retrieved wholesale, and the
//! agent's context pays the full cost whenever it wants any of it.
//!
//! This module rewrites such chains by inserting segment markers between the
//! component commands, and splits the produced output back into per-segment
//! pieces. `pre_tool_call` rewrites the command; `transform_tool_result`
//! detects the markers and compresses each segment independently, so the
//! agent sees N compact per-segment previews (`[chain:3 | build ... ]`) and
//! retrieves exactly the segment it needs.
//!
//! Safety: markers are plain `echo` statements that never alter command
//! semantics (exit codes, streams, side effects). A chain with zero or one
//! segment, or containing a marker-resisting construct (heredoc, single
//! quotes spanning the separator, trailing backslash continuation), is left
//! untouched.

/// Marker echoed between segments. Chosen to be visually distinct from real
/// output, unlikely in tool output, and greppable for tests.
pub const SEG_MARKER:&str = "__APHRODITE_SEG__";

/// A parsed chain segment: its raw command text and its index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
	pub index:usize,
	pub command:String,
}

/// Split a command string into segments on shell separators (`&&`, `;`,
/// newlines), respecting single/double quotes and trailing backslash
/// continuations. Returns `None` for constructs we must not rewrite.
pub fn split_chain(command:&str) -> Option<Vec<Segment>> {
	let mut segments = Vec::new();
	let mut cur = String::new();
	let mut chars = command.chars().peekable();
	let mut in_single = false;
	let mut in_double = false;
	let mut seg_index = 0usize;

	while let Some(c) = chars.next() {
		match c {
			'\'' if !in_double => in_single = !in_single,
			'"' if !in_single => in_double = !in_double,
			'\\' => {
				// Backslash continuation: swallow the next char (newline or
				// escaped char) so a multi-line command stays one segment.
				if let Some(n) = chars.next() {
					cur.push('\\');
					cur.push(n);
				}
				continue;
			},
			'&' if !in_single && !in_double => {
				// `&&` separator; a lone `&` (async) is left alone.
				if chars.peek() == Some(&'&') {
					chars.next();
					push_segment(&mut segments, &mut cur, seg_index);
					seg_index += 1;
					continue;
				}
				cur.push(c);
				continue;
			},
			';' if !in_single && !in_double => {
				push_segment(&mut segments, &mut cur, seg_index);
				seg_index += 1;
				continue;
			},
			'\n' if !in_single && !in_double => {
				// Newline between commands is a separator; a trailing
				// backslash already merged the next line into `cur`.
				push_segment(&mut segments, &mut cur, seg_index);
				seg_index += 1;
				continue;
			},
			_ => {},
		}
		cur.push(c);
	}
	push_segment(&mut segments, &mut cur, seg_index);

	// Require at least 2 non-empty segments to be worth rewriting.
	let segments:Vec<Segment> = segments.into_iter().filter(|s| !s.command.trim().is_empty()).collect();
	if segments.len() < 2 {
		return None;
	}
	// Refuse chains containing a heredoc marker or a `for`/`while` loop
	// spanning segments - rewriting those would corrupt control flow.
	for s in &segments {
		let lower = s.command.to_lowercase();
		if lower.contains("<<") || lower.contains("for ") || lower.contains("while ") {
			return None;
		}
	}
	Some(segments)
}

fn push_segment(segments:&mut Vec<Segment>, cur:&mut String, index:usize) {
	if !cur.trim().is_empty() {
		segments.push(Segment { index, command:cur.trim().to_string() });
	}
	cur.clear();
}

/// Build the rewritten command: each segment preceded by a marker echoed to
/// STDERR (not stdout). Markers on stderr never pollute stdout (files, pipes,
/// captured tool output stay clean); Hermes merges stderr into the terminal
/// result, so the transform hook still sees them and splits per segment.
pub fn build_marked_command(segments:&[Segment]) -> String {
	let mut out = String::new();
	for (i, seg) in segments.iter().enumerate() {
		if i > 0 {
			out.push_str(&format!("echo {} 1>&2 ; ", SEG_MARKER));
		}
		out.push_str(&seg.command);
		out.push_str(" ; ");
	}
	// Strip the trailing " ; ".
	if out.ends_with(" ; ") {
		out.truncate(out.len() - 3);
	}
	out
}

/// Split tool output on segment markers into (segment_index, text) pairs.
/// Output before the first marker belongs to segment 0. Marker lines are
/// removed; surrounding blank lines are trimmed per segment.
pub fn split_marked_output(output:&str) -> Vec<(usize, String)> {
	let mut parts:Vec<(usize, String)> = Vec::new();
	let mut cur = String::new();
	let mut cur_idx = 0usize;

	for line in output.lines() {
		let trimmed = line.trim();
		if trimmed == SEG_MARKER {
			if !cur.trim().is_empty() {
				parts.push((cur_idx, cur.trim().to_string()));
			}
			cur_idx += 1;
			cur = String::new();
		} else {
			cur.push_str(line);
			cur.push('\n');
		}
	}
	if !cur.trim().is_empty() {
		parts.push((cur_idx, cur.trim().to_string()));
	}
	parts
}

/// Extract a compact error hint from a segment's output, when it carries an
/// error signal (exit code, error:/Error:, FAILED/failed, panic, Traceback,
/// etc.). Returns the first matching line, trimmed to ≤60 chars.
///
/// Tier 3: per-segment error hints. This is CONTENT-level vocabulary (what
/// the segment itself says), never mechanism vocabulary - the invisibility
/// contract limits hints to the segment's own output, so the LLM learns
/// "that segment failed" from the hint without any chain/split wording.
pub fn segment_error_hint(content:&str) -> Option<String> {
	let mut first_signal:Option<&str> = None;
	for line in content.lines() {
		let l = line.trim();
		if l.is_empty() {
			continue;
		}
		let lower = l.to_lowercase();
		let is_error = l.contains("exit code:")
			|| lower.contains("error")
			|| lower.contains("failed")
			|| lower.contains("failure")
			|| lower.contains("panic")
			|| lower.contains("traceback")
			|| lower.contains("fatal")
			|| lower.contains("cannot")
			|| lower.contains("not found")
			|| lower.contains("no such file")
			|| lower.contains("permission denied")
			|| lower.contains("unresolved import")
			|| lower.contains("undefined");
		if !is_error {
			continue;
		}
		// `error[EXXXX]: msg` is the most informative shape - take it and
		// stop; otherwise remember the first signal and keep scanning for a
		// richer one.
		if l.contains("error[") {
			return Some(l.chars().take(60).collect());
		}
		if first_signal.is_none() {
			first_signal = Some(l);
		}
	}
	first_signal.map(|l| l.chars().take(60).collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn splits_on_and_and_semicolon() {
		let segs = split_chain("cd x && cargo build ; cargo test").unwrap();
		assert_eq!(segs.len(), 3);
		assert_eq!(segs[0].command, "cd x");
		assert_eq!(segs[1].command, "cargo build");
		assert_eq!(segs[2].command, "cargo test");
	}

	#[test]
	fn respects_quotes() {
		let segs = split_chain("echo 'a && b' ; echo \"c ; d\"").unwrap();
		assert_eq!(segs.len(), 2);
		assert_eq!(segs[0].command, "echo 'a && b'");
		assert_eq!(segs[1].command, "echo \"c ; d\"");
	}

	#[test]
	fn single_segment_is_none() {
		assert!(split_chain("cargo build").is_none());
		assert!(split_chain("").is_none());
	}

	#[test]
	fn heredoc_refused() {
		assert!(split_chain("cat << EOF && echo hi").is_none());
	}

	#[test]
	fn marked_command_roundtrip() {
		let segs = split_chain("a && b && c").unwrap();
		let marked = build_marked_command(&segs);
		assert!(marked.contains("__APHRODITE_SEG__"));
		// Markers go to STDERR, never stdout: redirected files stay clean.
		assert!(marked.contains("1>&2"));
		assert!(!marked.contains("echo __APHRODITE_SEG__ ;"));
		let out = format!("seg a output\n{}\nseg b output\n{}\nseg c output\n", SEG_MARKER, SEG_MARKER);
		let parts = split_marked_output(&out);
		assert_eq!(parts.len(), 3);
		assert_eq!(parts[0], (0, "seg a output".to_string()));
		assert_eq!(parts[1], (1, "seg b output".to_string()));
		assert_eq!(parts[2], (2, "seg c output".to_string()));
	}

	#[test]
	fn newline_separator() {
		let segs = split_chain("cd /tmp\ngit status\necho done").unwrap();
		assert_eq!(segs.len(), 3);
	}

	// ── Tier 3: per-segment error hints ──

	#[test]
	fn error_hint_none_for_clean_output() {
		assert_eq!(segment_error_hint("everything fine\nno problems here"), None);
		assert_eq!(segment_error_hint(""), None);
	}

	#[test]
	fn error_hint_finds_first_signal() {
		let out = "compiling...\nerror: could not compile `demo`\nfailed\n";
		assert_eq!(segment_error_hint(out).as_deref(), Some("error: could not compile `demo`"));
	}

	#[test]
	fn error_hint_prefers_error_code_line() {
		let out = "warning: unused\nfatal: something\nerror[E0432]: unresolved import `x`\n";
		assert_eq!(segment_error_hint(out).as_deref(), Some("error[E0432]: unresolved import `x`"));
	}

	#[test]
	fn error_hint_catches_exit_code() {
		assert_eq!(segment_error_hint("done\n").as_deref(), None);
		assert_eq!(
			segment_error_hint("make: *** [all] Error 2\n").as_deref(),
			Some("make: *** [all] Error 2")
		);
	}

	#[test]
	fn error_hint_truncates_long_lines() {
		let long = format!("error: {}", "x".repeat(200));
		let hint = segment_error_hint(&long).unwrap();
		assert!(hint.len() <= 60);
		assert!(hint.starts_with("error: "));
	}
}
