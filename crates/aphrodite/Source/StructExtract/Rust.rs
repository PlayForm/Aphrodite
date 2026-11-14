//! Rust extractor: `fn` / `struct` / `trait` / `impl` signatures.

use std::collections::HashMap;

use super::{MAX_SIG_LEN, floor_boundary, sig, trunc_params};

// ── Rust extractor ─────────────────────────────────────

pub(super) fn extract_rust(content:&str, result:&mut HashMap<String, Vec<String>>, budget:&mut isize) {
	// fn (with return type)
	let mut fns:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		let lower = trimmed.to_lowercase();

		let is_fn = (lower.starts_with("fn ")
			|| lower.starts_with("pub fn ")
			|| lower.starts_with("async fn ")
			|| lower.starts_with("pub async fn ")
			|| lower.starts_with("pub(crate) fn ")
			// T10 (F8): `pub(crate) async fn` was previously invisible here -
			// the strip chain a few lines below already handles stripping
			// "pub(crate) " then "async " in either order, but the `is_fn`
			// prefix check itself only recognized the non-async
			// `pub(crate) fn ` form.
			|| lower.starts_with("pub(crate) async fn "))
			&& trimmed.contains('(');

		if is_fn {
			// Extract name and params
			let after_fn = trimmed
				.trim_start_matches("pub(crate) ")
				.trim_start_matches("pub ")
				.trim_start_matches("async ")
				.trim_start_matches("fn ");
			if let Some(paren) = after_fn.find('(') {
				let name = &after_fn[..paren];
				let rest = &after_fn[paren..];
				let params_end = rest.find(')').unwrap_or(rest.len());
				let params = &rest[1..params_end];
				let ret = if rest[params_end..].contains("->") {
					rest[params_end..]
						.split("->")
						.nth(1)
						.unwrap_or("")
						.split('{')
						.next()
						.unwrap_or("")
						.trim()
				} else {
					""
				};
				let params_trunc = trunc_params(params);
				let ret_str = if ret.is_empty() { String::new() } else { format!(" -> {}", ret) };
				let s = format!("fn {}({}){}", name, params_trunc, ret_str);
				let s = if s.len() > MAX_SIG_LEN {
					floor_boundary(&s, MAX_SIG_LEN - 3).to_string() + "..."
				} else {
					s
				};
				let slen = s.len();
				fns.push(s);
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

	// struct
	let mut structs:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		let lower = trimmed.to_lowercase();
		if (lower.starts_with("struct ") || lower.starts_with("pub struct ")) && !trimmed.contains('(') {
			let name = trimmed
				.trim_start_matches("pub ")
				.trim_start_matches("struct ")
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.next()
				.unwrap_or("?");
			let s = sig("struct", name);
			let slen = s.len();
			structs.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !structs.is_empty() {
		result.insert("structs".into(), structs);
	}
	if *budget <= 0 {
		return;
	}

	// trait
	let mut traits:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		let lower = trimmed.to_lowercase();
		if lower.starts_with("trait ") || lower.starts_with("pub trait ") {
			let name = trimmed
				.trim_start_matches("pub ")
				.trim_start_matches("trait ")
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.next()
				.unwrap_or("?");
			let s = sig("trait", name);
			let slen = s.len();
			traits.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !traits.is_empty() {
		result.insert("traits".into(), traits);
	}
	if *budget <= 0 {
		return;
	}

	// impl
	let mut impls:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		let lower = trimmed.to_lowercase();
		if lower.starts_with("impl ") || lower.starts_with("impl<") {
			let name = trimmed
				.trim_start_matches("impl")
				.trim_start_matches('<')
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.find(|s| !s.is_empty())
				.unwrap_or("?");
			let s = sig("impl", name);
			let slen = s.len();
			impls.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !impls.is_empty() {
		result.insert("impls".into(), impls);
	}
}
