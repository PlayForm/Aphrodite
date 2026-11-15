//! `Stat::Fn`: compute the line statistics.

use super::Struct::Struct;

/// Compute the leading excerpt, the error/warning tallies and the first
/// diagnostic line.
///
/// `Lower` is passed in rather than recomputed so the caller pays for the
/// lowercased copy once - it is the same vector the message search walks. The
/// excerpt is capped at 110 CHARACTERS, not bytes, so a multi-byte line cannot
/// be split mid-character and render as a replacement glyph.
pub(crate) fn Fn(Content:&str, Lines:&[&str], Lower:&[String]) -> Struct {
	let First = Content
		.chars()
		.take(110)
		.collect::<String>()
		.replace(['\n', '\r'], " ")
		.trim()
		.to_string();
	let ErrorCount = Lower.iter().filter(|l| l.contains("error")).count();
	let WarningCount = Lower.iter().filter(|l| l.contains("warning")).count();
	let Message = Lines
		.iter()
		.zip(Lower.iter())
		.find(|(_, l)| l.contains("error") || l.contains("failed"))
		.map(|(l, _)| l.trim().chars().take(110).collect::<String>())
		.unwrap_or_default();
	Struct { FirstLine:First, ErrorCount, WarningCount, Message }
}
