//! Code structure extractor - regex-based pattern matching per language.
//! Port of plugins/aphrodite/_core/struct.py
//!
//! Extracts function/struct/class signatures from source code with
//! a 300-char preview budget. Used by the preview engine to show
//! code structure in CCR markers like [code_rust:3fns 2structs].

use std::collections::HashMap;

/// Maximum total output in characters (preview budget).
const BUDGET: usize = 300;

/// Maximum length of a single signature line.
const MAX_SIG_LEN: usize = 60;

/// Maximum param string length before truncation.
const MAX_PARAMS_LEN: usize = 35;

/// Byte-safe prefix of `s` no longer than `max` bytes - never splits a
/// multi-byte UTF-8 character, unlike a raw `&s[..max]` (which panics if
/// `max` falls inside a codepoint; source content is arbitrary UTF-8, not
/// guaranteed ASCII-aligned at any fixed offset).
pub(crate) fn floor_boundary(s: &str, max: usize) -> &str {
	if s.len() <= max {
		return s;
	}
	let mut i = max;
	while !s.is_char_boundary(i) {
		i -= 1;
	}
	&s[..i]
}

/// Extract code structure from source content.
/// Auto-detects language from content prefixes.
/// Returns a map of category → list of short signature strings.
pub fn extract_code_structure(content: &str, language: &str) -> HashMap<String, Vec<String>> {
	let lang = if language.is_empty() { auto_detect(content) } else { language.to_string() };

	if lang.is_empty() {
		return HashMap::new();
	}

	let mut result: HashMap<String, Vec<String>> = HashMap::new();
	let mut budget = BUDGET as isize;

	match lang.as_str() {
		"rust" => extract_rust(content, &mut result, &mut budget),
		"python" => extract_python(content, &mut result, &mut budget),
		"go" => extract_go(content, &mut result, &mut budget),
		"js" | "ts" => extract_js(content, &mut result, &mut budget),
		_ => {},
	}

	result
}

/// Number of non-comment lines `auto_detect` scans before giving up.
const AUTO_DETECT_SCAN_LINES: usize = 60;

/// Is this line a comment/doc-comment line, for the purposes of skipping
/// leading file-header comment blocks in `auto_detect`?
fn is_comment_line(trimmed: &str) -> bool {
	trimmed.is_empty()
		|| trimmed.starts_with("//") // Rust/Go/JS/TS line comments (incl. `///`, `//!`)
		|| trimmed.starts_with('#') // Python/shell comments (and shebangs)
		|| trimmed.starts_with('*') // continuation line of a `/* ... */` block
		|| trimmed.starts_with("/*")
}

/// Auto-detect the source language from content, by scanning up to
/// [`AUTO_DETECT_SCAN_LINES`] non-comment lines rather than a fixed 500-BYTE
/// prefix of the raw content (T10/F8): a file that opens with a long `//!`
/// module doc comment (extremely common in this very codebase) could push
/// every real `fn`/`struct`/etc. keyword past that byte window, so the
/// detector saw only comment prose and returned "unknown" for a file that is
/// unambiguously Rust. Skipping comment lines while scanning fixes that
/// without needing a real tokenizer.
fn auto_detect(content: &str) -> String {
	let sample: String = content
		.lines()
		.filter(|l| !is_comment_line(l.trim()))
		.take(AUTO_DETECT_SCAN_LINES)
		.collect::<Vec<_>>()
		.join("\n");
	let head = sample.as_str();
	if head.contains("fn ") && head.contains("->") {
		"rust".into()
	} else if head.contains("def ") && head.contains(":") {
		"python".into()
	} else if head.contains("func ") && head.contains("{") {
		"go".into()
	} else if head.contains("function ") || head.contains("=>") || head.contains("interface ") {
		"js".into()
	} else if content.trim_start().starts_with("#!/")
		|| (head.len() > 200 && floor_boundary(head, 200).contains("echo "))
	{
		"sh".into()
	} else {
		String::new()
	}
}

/// Truncate a signature to fit the preview budget.
fn sig(kind: &str, text: &str) -> String {
	let s = format!("{} {}", kind, text).trim().to_string();
	if s.len() > MAX_SIG_LEN {
		floor_boundary(&s, MAX_SIG_LEN - 3).to_string() + "..."
	} else {
		s
	}
}

fn trunc_params(params: &str) -> String {
	if params.len() > MAX_PARAMS_LEN {
		floor_boundary(params, MAX_PARAMS_LEN - 3).to_string() + "..."
	} else {
		params.to_string()
	}
}

// ── Rust extractor ─────────────────────────────────────

fn extract_rust(content: &str, result: &mut HashMap<String, Vec<String>>, budget: &mut isize) {
	// fn (with return type)
	let mut fns: Vec<String> = Vec::new();
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
	let mut structs: Vec<String> = Vec::new();
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
				.split(|c: char| c.is_whitespace() || c == '<' || c == '{')
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
	let mut traits: Vec<String> = Vec::new();
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
				.split(|c: char| c.is_whitespace() || c == '<' || c == '{')
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
	let mut impls: Vec<String> = Vec::new();
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
				.split(|c: char| c.is_whitespace() || c == '<' || c == '{')
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

// ── Python extractor ───────────────────────────────────

fn extract_python(content: &str, result: &mut HashMap<String, Vec<String>>, budget: &mut isize) {
	let mut fns: Vec<String> = Vec::new();
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

	let mut classes: Vec<String> = Vec::new();
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

// ── Go extractor ───────────────────────────────────────

fn extract_go(content: &str, result: &mut HashMap<String, Vec<String>>, budget: &mut isize) {
	let mut fns: Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		if trimmed.starts_with("func ") && trimmed.contains('(') {
			// func Name(...) or func (r *Receiver) Name(...)
			let after_func = &trimmed["func ".len()..];
			// T10 (F8): for a method, the receiver's own `(...)` comes before
			// the method's real parameter list - searching for the first `(`
			// in the whole line (as the old code did) finds the RECEIVER's
			// paren, not the params, so `func (s *Server) Start(addr string)`
			// used to report `func Start(s *Server)` instead of
			// `func Start(addr string)`. Search for the params paren only
			// after skipping past the receiver's closing `)`.
			let receiver_end = if after_func.starts_with('(') {
				after_func.find(')').map(|i| i + 1)
			} else {
				None
			};
			let name = match receiver_end {
				Some(end) => after_func[end..].trim_start().split('(').next().unwrap_or("?"),
				None => after_func.split('(').next().unwrap_or("?"),
			};
			let search_from = match receiver_end {
				// Absolute offset into `trimmed`: "func ".len() + receiver_end
				Some(end) => "func ".len() + end,
				None => 0,
			};
			let params_start = search_from + trimmed[search_from..].find('(').unwrap_or(0);
			let params_end = trimmed[params_start..].find(')').unwrap_or(0);
			let params = if params_end > 1 {
				&trimmed[params_start + 1..params_start + params_end]
			} else {
				""
			};
			let s = format!("func {}({})", name, trunc_params(params));
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
	if !fns.is_empty() {
		result.insert("fns".into(), fns);
	}
	if *budget <= 0 {
		return;
	}

	let mut types: Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		if trimmed.starts_with("type ") && trimmed.contains("struct") {
			let name = trimmed["type ".len()..].split("struct").next().unwrap_or("?").trim();
			let s = sig("type", name);
			let slen = s.len();
			types.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !types.is_empty() {
		result.insert("types".into(), types);
	}
}

// ── JS/TS extractor ────────────────────────────────────

fn extract_js(content: &str, result: &mut HashMap<String, Vec<String>>, budget: &mut isize) {
	let mut fns: Vec<String> = Vec::new();
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
		if !code_part.trim().is_empty() {
			if let Some((before_eq, after_eq)) = code_part.split_once('=') {
				let before_eq = before_eq.trim();
				if after_eq.contains("=>")
					&& (before_eq.starts_with("const ")
						|| before_eq.starts_with("let ")
						|| before_eq.starts_with("var "))
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
	}
	if !fns.is_empty() {
		result.insert("fns".into(), fns);
	}
	if *budget <= 0 {
		return;
	}

	let mut classes: Vec<String> = Vec::new();
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_auto_detect_rust() {
		assert_eq!(auto_detect("fn main() -> i32 {}\npub struct Foo {}"), "rust");
	}

	#[test]
	fn test_auto_detect_python() {
		assert_eq!(auto_detect("def hello():\n    pass\n"), "python");
	}

	#[test]
	fn test_auto_detect_go() {
		assert_eq!(auto_detect("func main() {\n}\n"), "go");
	}

	#[test]
	fn test_auto_detect_js() {
		assert_eq!(auto_detect("function hello() {\n}\n"), "js");
	}

	#[test]
	fn test_auto_detect_unknown() {
		assert_eq!(auto_detect("plain text no code"), "");
	}

	#[test]
	fn test_extract_rust_fns() {
		let code = "pub fn main() -> i32 {\n    42\n}\nfn helper(x: i32) -> bool {\n    true\n}\n";
		let r = extract_code_structure(code, "rust");
		assert!(r.contains_key("fns"));
		let fns = &r["fns"];
		assert!(fns.iter().any(|s| s.contains("main")));
		assert!(fns.iter().any(|s| s.contains("helper")));
	}

	#[test]
	fn test_extract_rust_structs() {
		let code = "pub struct Foo {\n    x: i32,\n}\nstruct Bar<T> {}\n";
		let r = extract_code_structure(code, "rust");
		assert!(r.contains_key("structs"));
	}

	#[test]
	fn test_extract_python() {
		let code = "def hello(name: str) -> str:\n    return name\n\nclass MyClass:\n    pass\n";
		let r = extract_code_structure(code, "python");
		assert!(r.contains_key("fns"));
		assert!(r.contains_key("classes"));
	}

	#[test]
	fn test_extract_go() {
		let code = "func main() {\n}\n\nfunc (s *Server) Start(addr string) error {\n}\n";
		let r = extract_code_structure(code, "go");
		assert!(r.contains_key("fns"));
	}

	// ── T10 (F8): per-language extractor correctness fixes ────────

	/// A method's receiver paren `(s *Server)` must not be mistaken for the
	/// method's actual parameter list - `params_start` used to find the
	/// FIRST `(` on the line (the receiver's), reporting
	/// `func Start(s *Server)` instead of `func Start(addr string)`.
	#[test]
	fn test_extract_go_method_reports_real_params_not_receiver() {
		let code = "func (s *Server) Start(addr string) error {\n}\n";
		let r = extract_code_structure(code, "go");
		let fns = &r["fns"];
		let sig = fns
			.iter()
			.find(|s| s.contains("Start"))
			.expect("Start method should be extracted");
		assert!(sig.contains("addr"), "signature should show the real params: {sig}");
		assert!(
			!sig.contains("*Server"),
			"signature should NOT show the receiver as if it were a param: {sig}"
		);
	}

	/// `const x = 5; // map => y` must not be recorded as an arrow function -
	/// the `=>` is inside a trailing comment, not part of the binding's RHS.
	#[test]
	fn test_extract_js_arrow_fn_guard_ignores_comment_only_arrow() {
		let code = "const x = 5; // map => y\n";
		let r = extract_code_structure(code, "js");
		if let Some(fns) = r.get("fns") {
			assert!(
				!fns.iter().any(|s| s.contains('x')),
				"trailing-comment `=>` must not be mistaken for an arrow fn: {fns:?}"
			);
		}
	}

	/// A real arrow function assignment must still be detected.
	#[test]
	fn test_extract_js_arrow_fn_still_detected() {
		let code = "const add = (a, b) => a + b;\n";
		let r = extract_code_structure(code, "js");
		let fns = &r["fns"];
		assert!(
			fns.iter().any(|s| s.contains("add")),
			"real arrow fn should still be extracted: {fns:?}"
		);
	}

	/// `pub(crate) async fn` was previously invisible to the Rust extractor's
	/// `is_fn` prefix check (only the non-async `pub(crate) fn` was listed).
	#[test]
	fn test_extract_rust_pub_crate_async_fn() {
		let code = "pub(crate) async fn tick(&self) -> bool {\n    true\n}\n";
		let r = extract_code_structure(code, "rust");
		let fns = r.get("fns").expect("pub(crate) async fn should be extracted");
		assert!(fns.iter().any(|s| s.contains("tick")), "expected tick() in {fns:?}");
	}

	/// A Rust file that opens with a long `//!` module-doc comment block
	/// (extremely common in this codebase) must still auto-detect as Rust -
	/// the old byte-prefix-based `auto_detect` could push every real
	/// keyword-bearing line past its fixed 500-byte window.
	#[test]
	fn test_auto_detect_rust_survives_long_leading_doc_comment() {
		let mut content = String::new();
		for i in 0..40 {
			content.push_str(&format!(
				"//! This is a long module doc comment line number {i} padding it out.\n"
			));
		}
		content.push_str("fn real_function() -> i32 { 42 }\n");
		assert_eq!(auto_detect(&content), "rust");
	}

	#[test]
	fn test_budget_respected() {
		// Generate lots of functions to test budget
		let mut code = String::new();
		for i in 0..50 {
			code.push_str(&format!("fn func{}(x: i32, y: i32, z: i32) -> i32 {{ 42 }}\n", i));
		}
		let r = extract_code_structure(&code, "rust");
		// Should have stopped before 50 due to budget
		if let Some(fns) = r.get("fns") {
			assert!(fns.len() < 50, "budget should cap output: got {}", fns.len());
		}
	}

	// ── T1 (F2): multi-byte UTF-8 must never panic a byte-offset truncation ──
	#[test]
	fn test_floor_boundary_never_panics_on_multibyte() {
		assert_eq!(floor_boundary("hello", 10), "hello");
		let s = "é".repeat(200); // every char is 2 bytes; any odd offset is mid-char
		let out = floor_boundary(&s, 99);
		assert!(out.len() <= 99);
		assert!(s.is_char_boundary(out.len()));
	}

	#[test]
	fn test_auto_detect_multibyte_near_500_byte_boundary() {
		// 'é' is 2 bytes; place one straddling byte offset 500.
		let mut content = "x".repeat(499);
		content.push('é');
		content.push_str("fn f() -> i32 { 1 }");
		// Must not panic.
		let _ = extract_code_structure(&content, "");
	}

	#[test]
	fn test_sig_multibyte_signature_does_not_panic() {
		let name = "é".repeat(40);
		// Must not panic when truncating a signature full of multi-byte chars.
		let s = sig("fn", &name);
		assert!(s.len() <= MAX_SIG_LEN + 3); // +3 for "..."
	}
}
