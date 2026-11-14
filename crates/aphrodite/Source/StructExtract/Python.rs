//! Python extractor: `def` / `class` signatures.

use std::collections::HashMap;

use super::{MAX_SIG_LEN, floor_boundary, sig, trunc_params};

// ── Python extractor ───────────────────────────────────

pub(super) fn extract_python(content:&str, result:&mut HashMap<String, Vec<String>>, budget:&mut isize) {
	let mut fns:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		let lower = trimmed.to_lowercase();
		let is_def = (lower.starts_with("def ") || lower.starts_with("async def ")) && trimmed.contains('(');
		if is_def {
			let after = trimmed.trim_start_matches("async ").trim_start_matches("def ");
			if let Some(paren) = after.find('(') {
				let name = &after[..paren];
				let params_end = after[paren..].find(')').unwrap_or(0);
				let params = if params_end > 1 { &after[paren + 1..paren + params_end] } else { "" };
				let s = format!("def {}({})", name, trunc_params(params));
				let s_trunc = if s.len() > MAX_SIG_LEN {
					floor_boundary(&s, MAX_SIG_LEN - 3).to_string() + "..."
				} else {
					s
				};
				let slen = s_trunc.len();
				fns.push(s_trunc);
				*budget -= slen as isize + 1;
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
			let name = rest.split(['(', ':']).next().unwrap_or("?");
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
