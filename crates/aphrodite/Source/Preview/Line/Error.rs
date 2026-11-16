//! Compiler/build error-line predicate.
//!
//! Error tallies have to be line-based, not substring-based: a word containing
//! "error" (`noerror`, `error-prone`) must never inflate a count, and a
//! capitalised variant must never be missed. This module is the single
//! definition of "this line is an error" that every preview arm shares, so the
//! build arm, the error arm and the linter arm cannot disagree about it.

/// True for a line that is a real compiler/build error line.
///
/// Recognises rustc/clang/gcc `error[E0432]:` and `error:`, a capitalised
/// `Error:`, an all-caps `ERROR`, Go's `panicked at`, `file:line: error[`-style
/// prefixes, and Python exception lines (`ValueError:`, `TypeError:`,
/// `Exception:`). It is not a substring match: the count that consumes this
/// predicate is presented as a tally, so a line that merely mentions an error
/// must not be counted as one.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	t.starts_with("error[")
		|| t.starts_with("error:")
		|| t.starts_with("Error:")
		|| t.starts_with("ERROR")
		|| t.starts_with("panicked at")
		|| t.contains(": error[")
		|| t.contains(": error:")
		|| Colon(t)
}

/// Token-before-colon scan.
///
/// For each `:` in the line, walk back over the ASCII word run; match when that
/// run ends `Error` or equals `Exception` (`ValueError:`, `KeyError:`,
/// `Exception:`). This replaces the old `\b\w+(?:Error|Exception):` regex and
/// deliberately keeps no hardcoded exception list - an exception type nobody
/// enumerated still matches.
fn Colon(t:&str) -> bool {
	let bytes = t.as_bytes();

	t.match_indices(':').any(|(i, _)| {
		// Walk back over the word run (ASCII alnum + `_`).
		let j = bytes[..i]
			.iter()
			.rposition(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
			.map_or(0, |p| p + 1);
		let Word = &t[j..i];

		Word.ends_with("Error") || Word == "Exception"
	})
}
