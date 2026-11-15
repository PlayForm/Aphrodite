//! Linter preview arm.
//!
//! A lint run's useful signal is its first issue line - a file, a position, a
//! code and a message. This arm surfaces that line. It is not a diagnostic
//! aggregator: it shows one issue, so a run with fifty findings previews as one
//! finding plus nothing, and the model has to retrieve the payload for the rest.

use crate::Preview::{Input::Struct::Struct as Input, Line::Lint};

/// Build a preview of linter output.
///
/// Promises the first issue line verbatim (`path:line:col: CODE message`, or an
/// `error:`/`warning:` line). It is not a count: no total of findings is
/// claimed, because the arms that count do so elsewhere and a wrong count reads
/// as a wrong result.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Hint = Inp
		.raw
		.lines()
		.map(|l| l.trim())
		.find(|l| Lint::Fn(l))
		.or_else(|| Inp.raw.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
		.map(|l| l.chars().take(60).collect::<String>());
	match Hint {
		Some(h) => format!("[lint:{}L {}B | {}]", Inp.Total, Inp.bytes, h),
		None => format!("[lint:{}L {}B]", Inp.Total, Inp.bytes),
	}
}
