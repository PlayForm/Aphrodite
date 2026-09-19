//! Compiler/build error-line predicate (honest line-based tallies).

/// True for a line that is a real compiler/build error line: rustc/clang/gcc
/// `error[E0432]:` / `error:`, capitalized `Error:`, all-caps `ERROR`, Go
/// `panicked at`, `file:line: error[`-style prefixes, and Python exception
/// lines (`ValueError:`, `TypeError:`, `Exception:`). Line-based (not
/// substring) counting so a word containing "error" (`noerror`, `error-prone`)
/// or a capitalized variant can never inflate/miss the tally.
pub(crate) fn is_error_line(line: &str) -> bool {
	let t = line.trim_start();
	t.starts_with("error[")
		|| t.starts_with("error:")
		|| t.starts_with("Error:")
		|| t.starts_with("ERROR")
		|| t.starts_with("panicked at")
		|| t.contains(": error[")
		|| t.contains(": error:")
		|| error_word_before_colon(t)
}

/// Token-before-colon scan (replaces the old `\b\w+(?:Error|Exception):`
/// regex, Phase 4): for each `:` in the line, walk back over the ASCII word
/// run; match when that run ends `Error` or equals `Exception` (e.g.
/// `ValueError:`, `KeyError:`, `Exception:`). No hardcoded exception list.
fn error_word_before_colon(t: &str) -> bool {
	let bytes = t.as_bytes();
	let mut idx = 0;
	while let Some(rel) = t[idx..].find(':') {
		let i = idx + rel;
		// Walk back over the word run (ASCII alnum + `_`).
		let mut j = i;
		while j > 0 && (bytes[j - 1].is_ascii_alphanumeric() || bytes[j - 1] == b'_') {
			j -= 1;
		}
		let word = &t[j..i];
		if word.ends_with("Error") || word == "Exception" {
			return true;
		}
		idx = i + 1;
	}
	false
}
