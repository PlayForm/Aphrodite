//! JS/TS extractor: `function` / arrow / `class` signatures.

use std::collections::HashMap;

use super::sig;

// ── JS/TS extractor ────────────────────────────────────

pub(super) fn extract_js(content:&str, result:&mut HashMap<String, Vec<String>>, budget:&mut isize) {
	let mut fns:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		// function name(...) or export function name(...) or async function name(...)
		if trimmed.starts_with("function ")
			|| trimmed.starts_with("export function ")
			|| trimmed.starts_with("async function ")
			|| trimmed.starts_with("export async function ")
		{
			let after = trimmed
				.trim_start_matches("export ")
				.trim_start_matches("async ")
				.trim_start_matches("function ");
			let name = after.split('(').next().unwrap_or("?").trim();
			let s = sig("function", name);
			let slen = s.len();
			fns.push(s);
			*budget -= slen as isize + 1;
		}
		// Arrow functions: const name = (...) => { ... }
		//
		// T10 (F8): strip a trailing `//` line comment first, THEN split
		// once on the first `=` and require the `=>` to appear on the
		// right-hand side of it - the old check (`trimmed.contains("=>") &&
		// trimmed.contains('=')`) matched `const x = 5; // map => y`,
		// mis-recording the constant `x` as an arrow function purely because
		// a `=>` happened to appear somewhere later in the line, inside a
		// trailing comment rather than the binding's actual value. A naive
		// fix that only special-cased a `//`-PREFIXED line (as opposed to a
		// trailing `// ...` after real code) would still miss this exact
		// case, since the line as a whole does not start with `//`.
		let code_part = trimmed.split("//").next().unwrap_or(trimmed);
		if !code_part.trim().is_empty()
			&& let Some((before_eq, after_eq)) = code_part.split_once('=')
		{
			let before_eq = before_eq.trim();
			if after_eq.contains("=>")
				&& (before_eq.starts_with("const ") || before_eq.starts_with("let ") || before_eq.starts_with("var "))
			{
				let name = before_eq
					.trim_start_matches("const ")
					.trim_start_matches("let ")
					.trim_start_matches("var ")
					.trim();
				if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
					let s = sig("=>", name);
					let slen = s.len();
					fns.push(s);
					*budget -= slen as isize + 1;
				}
			}
		}
	}
	if !fns.is_empty() {
		result.insert("fns".into(), fns);
	}
	if *budget <= 0 {
		return;
	}

	let mut classes:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		if let Some(rest) = trimmed.strip_prefix("class ") {
			let name = rest.split(['{', ' ', ':']).next().unwrap_or("?");
			let s = sig("class", name);
			let slen = s.len();
			classes.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !classes.is_empty() {
		result.insert("classes".into(), classes);
	}
}
