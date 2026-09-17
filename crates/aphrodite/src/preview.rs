//! Preview builder: content-type detection and human-readable preview strings
//! for compressed CCR content.
//!
//! Used by both the proxy binary and the `aphrodite-hermes` bridge crate
//! (via `crate::preview` re-export).

use headroom_core::transforms;

use serde_json::Value as JsonValue;

/// Detect the CCR content-type string for a blob (e.g. `source_code`, `build`,
/// `json_array`). Thin wrapper over the Headroom classifier so downstream
/// crates (aphrodite-hermes) don't need a direct headroom-core dependency.
pub fn detect_type(content: &str) -> String {
	transforms::content_detector::detect_content_type(content)
		.content_type
		.as_str()
		.to_string()
}

/// Aphrodite-side semantic detector for COMMON tool-output shapes the vendored
/// Headroom classifier flattens to bare `text`/`terminal`. Runs entirely in
/// Aphrodite's own layer (the fork boundary is preserved - vendor/ is never
/// touched); callers use it to OVERRIDE the classified type before building a
/// preview, the same override pattern `hooks::transform_terminal_output`
/// already uses for shell traces.
///
/// Returns `Some(type)` for a recognized shape (`json`, `test`, `diff`,
/// `code`, `table`, `markdown`, `yaml`, `html`, `xml`, `csv`, `build`, `git`,
/// `ls`, `grep`, `gitlog`), or `None` to leave the classifier's own verdict in
/// place. Detection is deliberately conservative (line-prefix / marker
/// patterns, majority votes) so a random paragraph is never mis-tagged.
/// Char-boundary safe and panic-free on empty/NUL/multibyte input.
///
/// Issue #11 residual #4 (RC-C): structured content (diffs, code, tables,
/// yaml/xml/csv, JSON objects, build logs) used to land on the generic
/// first-line arm - the detectors below give each shape its semantic arm.
pub fn detect_semantic_type(content: &str) -> Option<&'static str> {
	let lines: Vec<&str> = content.lines().collect();
	if lines.is_empty() {
		return None;
	}
	let non_empty: Vec<&str> = lines.iter().map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
	if non_empty.is_empty() {
		return None;
	}

	// ── JSON: a strict parse of an object/array is the strongest possible
	// shape signal - run it FIRST so a JSON payload can never be hijacked by
	// a marker substring inside one of its string values. ──
	// Hermes wrapper envelopes (`output`/`exit_code`, `diff`, `error`,
	// `success`, `total_count`/`matches`, `content`/`total_lines`,
	// `result`/`message`/`found`/`preview`, skill_view `name`/`description`)
	// are EXCLUDED: their raw-JSON previews are deliberately the caller-visible
	// payload (WS1 full-content preview), and hiding e.g. an error message
	// behind a key listing would be a regression.
	if let Ok(v) = serde_json::from_str::<JsonValue>(content) {
		match v {
			JsonValue::Object(obj) => {
				if !is_envelope_json_object(&obj) {
					return Some("json");
				}
			},
			JsonValue::Array(_) => return Some("json"),
			_ => {},
		}
	}

	// ── test output: cargo/pytest/jest/go ──
	// A `test result:` / `N passed` / `=== RUN` / pytest summary line is a
	// strong, unambiguous signal even amid other noise. Extended for the
	// SHALLOW tail (residual #4): `running N tests` + `test X ... ok` lines
	// (a bare test log without a summary line) also count as test output.
	if content.contains("test result:")
		|| content.contains("=== RUN ")
		|| content.contains("--- FAIL:")
		|| content.contains("--- PASS:")
		|| content.lines().any(|l| has_number_before(l, "passed") || has_number_before(l, "failed"))
		|| content.lines().any(|l| has_number_after(l, "Tests:"))
		|| content.lines().any(|l| is_running_tests_line(l.trim_start()))
		|| content.lines().any(|l| is_test_result_line(l.trim_start()))
	{
		return Some("test");
	}

	// ── diff: git/unified headers or a hunk ──
	let has_diff_git = non_empty.iter().any(|l| l.trim_start().starts_with("diff --git "));
	let has_ab_headers = {
		let a = non_empty.iter().any(|l| l.trim_start().starts_with("--- "));
		let b = non_empty.iter().any(|l| l.trim_start().starts_with("+++ "));
		a && b
	};
	let has_hunk = {
		let h = non_empty.iter().any(|l| l.trim_start().starts_with("@@ "));
		let delta = non_empty
			.iter()
			.filter(|l| {
				let t = l.trim_start();
				(t.starts_with('+') && !t.starts_with("+++")) || (t.starts_with('-') && !t.starts_with("---"))
			})
			.count();
		h && delta >= 1
	};
	if has_diff_git || has_ab_headers || has_hunk {
		return Some("diff");
	}

	// ── code: a strong signature marker (`fn main(`, `def x(`, `struct X`,
	// `#include`, shebang) or >=2 statement-line votes (`use x::y;`,
	// `let x =`, `import x`, `return x`, ...). ──
	let code_strong = non_empty.iter().any(|l| CODE_STRONG_RE.is_match(l));
	let code_votes = non_empty.iter().filter(|l| CODE_VOTE_RE.is_match(l)).count();
	if code_strong || code_votes >= 2 {
		return Some("code");
	}

	// ── markdown table: >=2 `|`-prefixed lines with a separator row ──
	let md_table_lines = non_empty.iter().filter(|l| l.trim_start().starts_with('|')).count();
	let has_table_sep = non_empty.iter().any(|l| {
		let cells = l.trim().trim_matches('|');
		!cells.is_empty()
			&& cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
			})
	});
	if md_table_lines >= 2 && has_table_sep {
		return Some("table");
	}

	// ── markdown document: heading lines + list/table/link/fence/quote
	// structure or body prose. Gated so comment-only shell scripts cannot
	// trigger it: a doc needs structure OR non-heading body lines - a file of
	// bare `# comment` lines has neither. ──
	let md_heads = non_empty.iter().filter(|l| is_md_heading(l)).count();
	let md_structure = non_empty.iter().filter(|l| is_md_structure(l)).count();
	let md_body = non_empty.iter().filter(|l| !is_md_heading(l) && !is_md_structure(l)).count();
	if md_heads >= 2 && (md_structure >= 1 || md_body >= 1) {
		return Some("markdown");
	}

	// ── yaml: >=3 top-level lowercase-key lines (`name: webapp`). Log-marker
	// keys (`error:`/`warning:`/...) are excluded so a compiler log cannot be
	// mis-tagged as yaml. ──
	let yaml_keys = content.lines().filter(|l| is_yaml_key_line(l)).count();
	if yaml_keys >= 3 {
		return Some("yaml");
	}

	// ── html vs xml: a document that opens with `<!DOCTYPE html`/`<html` is
	// html (the dedicated arm extracts <title>); other tag documents with an
	// opening `<` and a `</` close count as xml. ──
	let trimmed = content.trim_start();
	if trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html") || trimmed.starts_with("<HTML") {
		return Some("html");
	}
	if trimmed.starts_with('<') && content.trim_end().ends_with('>') && content.contains("</") {
		return Some("xml");
	}

	// ── csv: >=2 rows with an identical comma-field count (>=2 fields) and no
	// `, ` (comma-space) - prose with commas is excluded by both tests. ──
	let csv_rows: Vec<&str> = non_empty.iter().map(|l| l.trim()).collect();
	if csv_rows.len() >= 2 && !content.contains(", ") {
		let counts: Vec<usize> = csv_rows.iter().map(|r| r.split(',').count()).collect();
		let first = counts[0];
		if first >= 2 && counts.iter().all(|&c| c == first) {
			return Some("csv");
		}
	}

	// ── build output: cargo/rustc-style verb lines (`Compiling`, `Finished`,
	// `error[`, `error:`, `warning:`, `-->`) - >=2 markers, or a verb plus an
	// error/warning line. A lone `error: broke` terminal trace does NOT count
	// (stays on the terminal arm). ──
	let build_verbs = non_empty
		.iter()
		.filter(|l| {
			let t = l.trim_start();
			["Compiling ", "Building ", "Finished ", "Checking ", "Linking ", "--> "]
				.iter()
				.any(|p| t.starts_with(p))
		})
		.count();
	let build_errs = non_empty
		.iter()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("error[")
				|| t.starts_with("error:")
				|| t.starts_with("Error:")
				|| t.starts_with("warning[")
				|| t.starts_with("warning:")
				|| t.starts_with("Warning:")
		})
		.count();
	if build_verbs + build_errs >= 2 {
		return Some("build");
	}

	// ── git status: porcelain / `M `/`A `/`D `/`R `/`??`/`UU` prefixes ──
	// Require a majority of non-empty lines to carry a status code so a diff
	// hunk (`+`/`-`) or prose isn't mistaken for a status listing.
	let status_lines = non_empty.iter().filter(|l| git_status_code(l).is_some()).count();
	if status_lines >= 2 && status_lines * 2 >= non_empty.len() {
		return Some("git");
	}

	// ── git log: `commit <hash>` blocks ──
	let commit_lines = non_empty
		.iter()
		.filter(|l| {
			l.strip_prefix("commit ")
				.map(|h| h.trim().len() >= 7 && h.trim().chars().take(7).all(|c| c.is_ascii_hexdigit()))
				.unwrap_or(false)
		})
		.count();
	if commit_lines >= 1 && (commit_lines >= 2 || content.contains("Author:")) {
		return Some("gitlog");
	}

	// ── grep/ripgrep: `path:line:match` or `path:match` majority ──
	let grep_lines = non_empty.iter().filter(|l| is_grep_line(l)).count();
	if grep_lines >= 2 && grep_lines * 2 >= non_empty.len() {
		return Some("grep");
	}

	// ── directory listing: `ls -l` mode strings, or a majority of bare
	// path-like tokens (find / plain ls). ──
	let ls_long = non_empty
		.iter()
		.filter(|l| {
			let b = l.as_bytes();
			b.len() >= 10
				&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
				&& b[1..10]
					.iter()
					.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'))
		})
		.count();
	if ls_long >= 2 {
		return Some("ls");
	}
	let path_lines = non_empty.iter().filter(|l| is_path_line(l)).count();
	if path_lines >= 3 && path_lines * 2 >= non_empty.len() {
		return Some("ls");
	}

	None
}

/// True when a JSON object is a Hermes wrapper envelope whose raw-JSON preview
/// is intentional (the payload is inside the wrapper, not the key list).
fn is_envelope_json_object(obj: &serde_json::Map<String, JsonValue>) -> bool {
	const GUARD: &[&str] = &[
		"output", "exit_code", "diff", "error", "success", "total_count", "matches", "matches_text", "content",
		"total_lines", "result", "message", "found", "preview", "name", "description",
	];
	obj.keys().any(|k| GUARD.contains(&k.as_str()))
}

/// True for a markdown heading line (`# `, `## `, ... `###### `).
fn is_md_heading(line: &str) -> bool {
	let t = line.trim_start();
	let n = t.chars().take_while(|c| *c == '#').count();
	n >= 1 && n <= 6 && t.len() > n && t[n..].starts_with(' ') && t[n..].trim().len() > 0
}

/// True for a non-heading markdown structural line (list item, link, fence,
/// blockquote, horizontal rule).
fn is_md_structure(line: &str) -> bool {
	let t = line.trim_start();
	t.starts_with("- ")
		|| t.starts_with("* ")
		|| t.starts_with("+ ")
		|| t.starts_with("> ")
		|| t.starts_with("```")
		|| t.starts_with("~~~")
		|| t.starts_with("|")
		|| t.starts_with("[") && t.contains("](")
		|| (!t.is_empty() && t.chars().all(|c| c == '-' || c == '=') && t.len() >= 3)
}

/// True for a top-level YAML key line (`name: webapp`): a lowercase
/// identifier key, no leading indent, non-empty value side allowed. Log-marker
/// keys are excluded so compiler logs are never mis-tagged as yaml.
fn is_yaml_key_line(line: &str) -> bool {
	if line.starts_with(' ') || line.starts_with('	') {
		return false;
	}
	let t = line.trim_end();
	let idx = match t.find(':') {
		Some(i) => i,
		None => return false,
	};
	if idx == 0 || idx > 64 {
		return false;
	}
	let key = &t[..idx];
	if !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
		return false;
	}
	!matches!(key, "error" | "warning" | "note" | "info" | "warn" | "debug" | "trace")
}

/// First non-empty line whose trimmed form is not structural noise (a lone
/// `{`/`}`/`[`/`]`/`(`/`)` with optional trailing `,`/`;`). Pretty-printed
/// JSON opens with a lone `{` - previewing that line alone is the ISSUE-11
/// residual #1 MISLEADING bug (RC-D: the generic arm showed the first line).
/// Skips to the first real content line (first key, first statement).
fn first_meaningful_line(content: &str) -> Option<String> {
	content.lines().find_map(|l| {
		let t = l.trim();
		if t.is_empty() {
			return None;
		}
		let core = t.trim_end_matches(|c| c == ',' || c == ';').trim();
		let mut it = core.chars();
		match (it.next(), it.next()) {
			(Some(c), None) if matches!(c, '{' | '}' | '[' | ']' | '(' | ')') => None,
			_ => Some(t.to_string()),
		}
	})
}

/// Render a content hint line: lines up to 100 chars are shown as-is
/// (60-char cap); very long lines (a 10 KB single-line payload) sample
/// head+tail (`head…tail`, 57 chars) so both ends are visible instead of a
/// bare 60-char head (ISSUE-11 residual #4, `long_single_line`/`repeated`).
fn sample_long_line(line: &str) -> String {
	let chars: Vec<char> = line.chars().collect();
	if chars.len() > 100 {
		let head: String = chars[..28].iter().collect();
		let tail: String = chars[chars.len() - 28..].iter().collect();
		format!("{head}…{tail}")
	} else {
		line.chars().take(60).collect()
	}
}

/// Git porcelain / short-status code for a line (`M `, ` M`, `A `, `D `, `R `,
/// `??`, `UU`, etc.), or `None`. Two leading columns (staged, unstaged) then a
/// space then a path.
fn git_status_code(line: &str) -> Option<&str> {
	let b = line.as_bytes();
	if b.len() < 4 {
		return None;
	}
	let codes = [b'M', b'A', b'D', b'R', b'C', b'U', b'?', b'!', b' ', b'T'];
	let c0 = b[0];
	let c1 = b[1];
	// Reject an all-space prefix (that's just indented prose).
	if (c0 == b' ' && c1 == b' ') || !codes.contains(&c0) || !codes.contains(&c1) {
		return None;
	}
	// Column 3 must be a space separating code from path, and a path follows.
	if b[2] != b' ' || line[3..].trim().is_empty() {
		return None;
	}
	// A two-char code of at least one real status letter (not `  `).
	line.get(..2)
}

/// True when a line looks like a grep/ripgrep hit: `path:line:match` (with a
/// numeric line field) or `path:match` where the path has a file-ish shape.
fn is_grep_line(line: &str) -> bool {
	let mut it = line.splitn(3, ':');
	let path = match it.next() {
		Some(p) if !p.trim().is_empty() && !p.contains(' ') => p,
		_ => return false,
	};
	// Path should look file-ish: contain a `/` or a `.ext`.
	if !path.contains('/') && !path.contains('.') {
		return false;
	}
	match it.next() {
		// `path:line:...` - middle is a line number.
		Some(mid) if mid.chars().all(|c| c.is_ascii_digit()) && !mid.is_empty() && it.next().is_some() => true,
		_ => false,
	}
}

/// True when a line is a bare path-like token (find output / plain `ls`): a
/// single whitespace-free token that has an extension or a path separator.
fn is_path_line(line: &str) -> bool {
	let t = line.trim();
	if t.is_empty() || t.contains(char::is_whitespace) {
		return false;
	}
	t.contains('/') || (t.rfind('.').map(|i| i > 0 && i < t.len() - 1).unwrap_or(false))
}

/// ── Structured detection matchers (Issue #11 residual #4; user directive:
/// the type-detection layer must NOT use the regex crate - explicit
/// `strip_prefix`/`starts_with` checks against literal prefixes, testable,
/// no raw-string escaping). ──

/// `running N tests` header (bare test logs without a summary line).
fn is_running_tests_line(t: &str) -> bool {
	let rest = match t.strip_prefix("running ") {
		Some(r) => r,
		None => return false,
	};
	let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
	if digits == 0 {
		return false;
	}
	let after = rest[digits..].trim_start();
	let after = match after.strip_prefix("test") {
		Some(a) => a,
		None => return false,
	};
	let after = match after.strip_prefix('s') {
		Some(a) => a,
		None => after,
	};
	// `tests?\b`: the word must end here or be followed by a non-word char.
	after.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true)
}

/// `test <name> ... ok|FAILED|ignored` line.
fn is_test_result_line(t: &str) -> bool {
	let rest = match t.strip_prefix("test ") {
		Some(r) => r,
		None => return false,
	};
	let name_len = rest.chars().take_while(|c| !c.is_whitespace()).count();
	if name_len == 0 {
		return false;
	}
	let after = rest[name_len..].trim_start();
	let after = match after.strip_prefix("...") {
		Some(a) => a,
		None => return false,
	};
	let after = after.trim_start();
	for kw in ["ok", "FAILED", "ignored"] {
		if let Some(r) = after.strip_prefix(kw) {
			// word boundary: next char is end or non-word.
			return r.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true);
		}
	}
	false
}

/// `N passed` / `N failed` anywhere on a line (pytest summaries).
fn has_number_before(line: &str, kw: &str) -> bool {
	match line.find(kw) {
		Some(i) => line[..i].trim_end().chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false),
		None => false,
	}
}

/// `Tests: N` jest summary line.
fn has_number_after(line: &str, kw: &str) -> bool {
	match line.find(kw) {
		Some(i) => line[i + kw.len()..].trim_start().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false),
		None => false,
	}
}

/// `fn name(` / `def name(` / `func name(` style signature.
fn is_fn_style_sig(t: &str, kw: &str) -> bool {
	let rest = match t.strip_prefix(kw) {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.trim_start();
	let name_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	name_len > 0 && rest[name_len..].trim_start().starts_with('(')
}

/// `struct Name` / `enum Name` / `trait Name` (optionally `pub`-prefixed).
fn is_type_decl(t: &str) -> bool {
	let stripped = t.strip_prefix("pub ").unwrap_or(t);
	for kw in ["struct ", "enum ", "trait "] {
		if let Some(rest) = stripped.strip_prefix(kw) {
			let name_len = rest.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
			if name_len > 0 {
				return true;
			}
		}
	}
	false
}

/// `#include <...>` / `#include "..."` / `#include<...>`.
fn is_include_directive(t: &str) -> bool {
	let rest = match t.strip_prefix("#include") {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.trim_start();
	rest.starts_with('<') || rest.starts_with('"')
}

/// Strong code-signature line: ONE such line is enough to call content code.
fn is_code_strong_line(t: &str) -> bool {
	is_fn_style_sig(t, "fn ")
		|| is_fn_style_sig(t, "def ")
		|| is_fn_style_sig(t, "func ")
		|| is_type_decl(t)
		|| t.starts_with("impl ")
		|| t.starts_with("class ")
		|| t.starts_with("#!")
		|| is_include_directive(t)
		|| t.strip_prefix("package ")
			.map(|r| r.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false))
			.unwrap_or(false)
}

/// `let x =` / `const X =` / `static X =` assignment (optional `mut`).
fn is_let_assign(t: &str) -> bool {
	let rest = match ["let ", "const ", "static "].iter().find_map(|p| t.strip_prefix(p)) {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim_start();
	let name_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').count();
	name_len > 0 && rest[name_len..].trim_start().starts_with('=')
}

/// `use std::collections::HashMap;` (rust use statement ending in `;`).
fn is_use_statement(t: &str) -> bool {
	let rest = match t.strip_prefix("use ") {
		Some(r) => r,
		None => return false,
	};
	let path_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').count();
	path_len > 0 && rest[path_len..].starts_with(';')
}

/// `from x import y` (python).
fn is_from_import(t: &str) -> bool {
	let rest = match t.strip_prefix("from ") {
		Some(r) => r,
		None => return false,
	};
	let mod_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	mod_len > 0 && rest[mod_len..].trim_start().starts_with("import")
}

/// Code statement-line vote: `use x::y;`, `let x =`, `import x`,
/// `from x import y`, `return ...`, `println!`, `print(`, `echo ...`.
fn is_code_vote_line(t: &str) -> bool {
	is_use_statement(t)
		|| is_let_assign(t)
		|| is_from_import(t)
		|| t.strip_prefix("import ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false))
			.unwrap_or(false)
		|| t.strip_prefix("return")
			.map(|r| r.chars().next().map(|c| c.is_whitespace()).unwrap_or(false))
			.unwrap_or(false)
		|| t.starts_with("println!")
		|| t.starts_with("print(")
		|| t.strip_prefix("print").map(|r| r.trim_start().starts_with('(')).unwrap_or(false)
		|| t.strip_prefix("echo ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '$').unwrap_or(false))
			.unwrap_or(false)
}

/// Process-wide preview length cap in chars; 0 = unlimited. Set from
/// `[previews] preview_max_chars` (Issue #11 WS4): the key was declared in
/// the config structs but never read anywhere, so every preview knob was a
/// no-op. The builder now enforces it on EVERY path - proxy, hooks, and the
/// Hermes dylib all funnel through [`build_preview`].
static PREVIEW_MAX_CHARS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Configure the preview length cap (chars); `None`/0 = unlimited.
/// Returns the previous value so callers (tests, hot-reload) can restore it.
///
/// Wiring (Issue #11 WS4): the engine binary sets it right after
/// `MultiConfig::load`, the proxy `/reload` handler re-sets it on hot-reload,
/// and the Hermes bridge applies it via
/// `config_loader::Config::apply_previews` at dylib init.
pub fn set_preview_max_chars(max: Option<u32>) -> Option<u32> {
	let prev = PREVIEW_MAX_CHARS.swap(max.unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
	if prev == 0 { None } else { Some(prev) }
}

/// Current preview cap in chars (0 = unlimited). Test/visibility helper.
pub fn preview_max_chars() -> u32 {
	PREVIEW_MAX_CHARS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Serializes tests across modules that mutate the process-global preview
/// cap (`cargo test` runs module test-bodies concurrently; the cap is
/// process-wide, so config_loader's `apply_previews` test and this module's
/// cap tests must not interleave).
#[cfg(test)]
pub(crate) fn preview_cap_test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Truncate a preview to `max_chars` chars on a char boundary. Preserves the
/// closing `]` (with a `…` marker) for self-bracketed previews so
/// `render_marker`/`parse_preview`/`chain_split` keep working after
/// truncation - a bare cut that drops `]` would re-trigger the
/// `[text:[text:...]]` double-wrap bug class (`marker.rs`). `0` = no cap.
fn apply_preview_cap(preview: &str, max_chars: usize) -> String {
	if max_chars == 0 || preview.chars().count() <= max_chars {
		return preview.to_string();
	}
	let keep_close = preview.trim_end().ends_with(']');
	if !keep_close || max_chars <= 1 {
		return preview.chars().take(max_chars).collect();
	}
	let mut out: String = preview.chars().take(max_chars - 2).collect();
	out.push('…');
	out.push(']');
	out
}

/// True for a line that is a real compiler/build error line: rustc/clang/gcc
/// `error[E0432]:` / `error:`, capitalized `Error:`, all-caps `ERROR`, Go
/// `panicked at`, `file:line: error[`-style prefixes, and Python exception
/// lines (`ValueError:`, `TypeError:`, `Exception:`). Line-based (not
/// substring) counting so a word containing "error" (`noerror`, `error-prone`)
/// or a capitalized variant can never inflate/miss the tally.
fn is_error_line(line: &str) -> bool {
	let t = line.trim_start();
	t.starts_with("error[")
		|| t.starts_with("error:")
		|| t.starts_with("Error:")
		|| t.starts_with("ERROR")
		|| t.starts_with("panicked at")
		|| t.contains(": error[")
		|| t.contains(": error:")
		|| ERROR_LINE_RE.is_match(t)
}

static ERROR_LINE_RE: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"\b\w+(?:Error|Exception):").unwrap());

/// True for a line that is a real compiler warning line (`warning[`/`warning:`
/// /`Warning:`/`WARNING` or a `file:line: warning:` prefix).
fn is_warning_line(line: &str) -> bool {
	let t = line.trim_start();
	t.starts_with("warning[")
		|| t.starts_with("warning:")
		|| t.starts_with("Warning:")
		|| t.starts_with("WARNING")
		|| t.contains(": warning[")
		|| t.contains(": warning:")
}

/// True for a line that signals a FAILED state: a `FAILED`/`FAIL` marker, a
/// `--- FAIL:` test failure, or a NON-ZERO `N failed` count. `0 failed`
/// (clean runs) never matches, so a passing test summary stays clean.
fn is_failure_line(line: &str) -> bool {
	let t = line.trim_start();
	t.contains("FAILED")
		|| t.starts_with("FAIL ")
		|| t.starts_with("--- FAIL:")
		|| FAILED_COUNT_RE.is_match(t)
}

static FAILED_COUNT_RE: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)[1-9]\d*\s+failed").unwrap());

/// True for a linter issue line: `path:line:col:` prefix (ruff/flake8/eslint)
/// or a `E###`/`W###`/`F###` issue code.
fn is_lint_line(line: &str) -> bool {
	let t = line.trim_start();
	is_error_line(t) || is_warning_line(t) || LINT_RE.is_match(t)
}

static LINT_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
	regex::Regex::new(r"^[^\s:]+:\d+:\d+:|(?:^|\s)[EWF]\d{3,4}\b").unwrap()
});

/// Build a compact, human-readable preview string for compressed content,
/// shaped per content type (e.g. error/warning counts for build output,
/// +/- line counts for diffs, fn/struct counts for source code) so the LLM
/// gets a useful summary instead of a generic byte/line count wherever a
/// richer signal is available.
pub fn build_preview(type_str: &str, content: &str) -> String {
	let lines = content.lines().count();
	let bytes = content.len();
	// Semantic-detection DEFAULT (no flag): when the classifier only reached a
	// generic bucket (`text`/`terminal`/`log`/`""`), let Aphrodite's own
	// detector upgrade the arm to a high-signal shape (git status, ls, test,
	// grep, git log, diff, code, table, yaml/xml/csv, json, build). Both the
	// proxy path and the Hermes hook/FFI path funnel through this one
	// function, so the enriched preview is emitted identically on both paths.
	// An explicit non-generic `type_str` is always honored as-is.
	//
	// ISSUE-11 residual #4 (RC-C): the `tool_result` hint is caller intent for
	// the STORED type, but the PREVIEW arm must still reflect content reality -
	// a raw diff / code block / table / yaml blob hinted `tool_result` used to
	// land on the generic first-line arm (SHALLOW). `build_output` keeps the
	// build arm except for a PASSING test run misclassified by the classifier
	// (`test result: ok.` - the build arm's clean `0E 0W` summary hides the
	// payload's real shape); a FAILING run keeps the build arm's failure note
	// (WS2 pin).
	let effective: &str = match type_str {
		"text" | "terminal" | "log" | "" | "plain" | "tool_result" => detect_semantic_type(content).unwrap_or(type_str),
		"build_output" | "build_error" => match detect_semantic_type(content) {
			Some("test") if !content.lines().any(|l| is_failure_line(l)) => "test",
			_ => type_str,
		},
		other => other,
	};
	let preview = match effective {
		"build" | "build_output" | "build_error" => {
			// Honest tallies (Issue #11 WS2): count error/warning LINES, not
			// substring occurrences. The old `content.matches("error").count()`
			// inflated lines with repeated occurrences, matched inside
			// unrelated words, and missed capitalized `Error:` (Python/Swift/
			// clang output) - a genuinely failed build could render as `0E`.
			let e = content.lines().filter(|l| is_error_line(l)).count();
			let w = content.lines().filter(|l| is_warning_line(l)).count();
			// Enrich: surface the first error MESSAGE (e.g. `E0432: unresolved
			// import ...`), not just tallies - the exact text the agent needs to
			// decide whether to retrieve the full log.
			let first_err = content
				.lines()
				.map(|l| l.trim())
				.find(|l| is_error_line(l))
				.map(|l| {
					// Prefer the `error[EXXXX]: msg` / `error: msg` remainder.
					let start = l
						.find("error")
						.or_else(|| l.find("Error"))
						.or_else(|| l.find("ERROR"))
						.unwrap_or(0);
					l[start..].chars().take(60).collect::<String>()
				});
			match first_err {
				Some(msg) if !msg.is_empty() => {
					format!("[build:{}E {}W {}L | {}]", e, w, lines, msg)
				},
				_ => {
					// Honesty: `0E 0W` renders like a clean build. When the
					// output carries a FAILURE signal (failing test run,
					// FAILED markers) but no error line was tallied, surface
					// the failure line instead of a success-looking summary -
					// a failing test run used to preview as
					// `[build:0E 0W 3L]` (ISSUE-11-PREVIEW-BATTERY #2).
					if e == 0 && w == 0 {
						if let Some(fail) = content
							.lines()
							.map(|l| l.trim())
							.find(|l| is_failure_line(l))
						{
							return format!(
								"[build:{}E {}W {}L | {}]",
								e,
								w,
								lines,
								fail.chars().take(60).collect::<String>()
							);
						}
					}
					format!("[build:{}E {}W {}L]", e, w, lines)
				},
			}
		},
		"diff" => {
			let f = content.matches("diff --git").count();
			let a = content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
			let d = content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
			// Enrich: name the first couple of changed files so the agent sees
			// WHAT changed, not just how many lines.
			let mut files: Vec<String> = content
				.lines()
				.filter_map(|l| l.strip_prefix("diff --git "))
				.filter_map(|rest| rest.split_whitespace().next())
				.map(|p| p.strip_prefix("a/").unwrap_or(p).to_string())
				.take(3)
				.collect();
			if files.is_empty() {
				// Unified `diff -u` / `git diff` without --git headers: name
				// files from `--- a/path` / `+++ b/path` header pairs so the
				// preview is never `0F` with no file context (residual #4).
				let mut seen = std::collections::BTreeSet::new();
				for l in content.lines() {
					let p = l.strip_prefix("--- ").or_else(|| l.strip_prefix("+++ "));
					if let Some(p) = p {
						let p = p.split_whitespace().next().unwrap_or("");
						let p = p.strip_prefix("a/").or_else(|| p.strip_prefix("b/")).unwrap_or(p);
						if !p.is_empty() && seen.insert(p.to_string()) && files.len() < 3 {
							files.push(p.to_string());
						}
					}
				}
			}
			if files.is_empty() {
				format!("[diff:{}F +{}/-{} {}L]", f, a, d, lines)
			} else {
				let more = if f > files.len() {
					format!(" +{} more", f - files.len())
				} else {
					String::new()
				};
				format!("[diff:{}F +{}/-{} {}L | {}{}]", f, a, d, lines, files.join(" "), more)
			}
		},
		// git status: staged/unstaged tallies by code + first few paths.
		"git" | "git_status" => build_git_status_preview(content, lines),
		// git log: commit count + first/last short hash and subject.
		"gitlog" | "git_log" => build_gitlog_preview(content, lines),
		// directory listing: file/dir counts + top extensions.
		"ls" | "dir" | "directory" => build_ls_preview(content, lines),
		// test output: pass/fail/ignored tallies + first failing test.
		"test" | "test_output" => build_test_preview(content, lines),
		// grep/ripgrep: hit count, files touched, first location.
		"grep" | "ripgrep" => build_grep_preview(content, lines),
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh" | "code" => {
			// Enrich with the structure map (fns/structs/traits/impls/classes/types
			// + first signature) so the dylib/hook path matches the proxy's preview
			// quality, instead of a bare substring count.
			let st = crate::struct_extract::extract_code_structure(content, "");
			let mut parts: Vec<String> = Vec::new();
			for (key, label) in [
				("fns", "fns"),
				("structs", "structs"),
				("traits", "traits"),
				("impls", "impls"),
				("classes", "classes"),
				("types", "types"),
			] {
				if let Some(v) = st.get(key) {
					if !v.is_empty() {
						parts.push(format!("{}{}", v.len(), label));
					}
				}
			}
			let summary = if parts.is_empty() {
				format!("{}fns", content.matches("fn ").count() + content.matches("def ").count())
			} else {
				parts.join("|")
			};
			let sig = st
				.get("fns")
				.and_then(|v| v.first())
				.map(|s| format!(" {}", s.chars().take(48).collect::<String>().trim()))
				.unwrap_or_default();
			format!("[code:{}{} {}L]", summary, sig, lines)
		},
		"search" => build_search_preview(content, lines),
		"html" => build_html_preview(content, lines),
		// ISSUE-11 residual #4: structured shapes that used to land on the
		// generic first-line arm (SHALLOW) - markdown tables, markdown docs,
		// yaml, xml, csv - each get a semantic arm with counts + a sample.
		"table" | "markdown_table" | "md_table" => build_table_preview(content, lines),
		"markdown" | "md" => build_markdown_preview(content, lines),
		"yaml" => build_yaml_preview(content, lines),
		"xml" => build_xml_preview(content, lines),
		"csv" => build_csv_preview(content, lines),
		"json_array" | "json" | "json_list" => build_json_preview(content, lines),
		// `hooks::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace, but
		// this function had no matching arm for it, so the preview silently
		// fell through to the generic `_` branch (F10) - a bare line/byte
		// count with no exit-code or last-output-line context, the exact
		// signal a terminal preview exists to surface.
		//
		// ISSUE-11 residual #2: the fallback used to be the LAST non-empty
		// line - a multi-line code block with a terminal hint previewed as
		// `[terminal:7L }]`, the closing brace. The FIRST meaningful line
		// (skipping lone braces) is the honest default; an `exit code:`/
		// `Error:` line still wins when present (most recent state signal).
		"terminal" => {
			let exit_line = content
				.lines()
				.rev()
				.find(|l| l.contains("exit code:") || l.contains("Error:"))
				.map(|l| l.trim());
			let summary = exit_line
				.or_else(|| first_meaningful_line(content).as_deref())
				.unwrap_or("")
				.chars()
				.take(60)
				.collect::<String>();
			format!("[terminal:{}L {}]", lines, summary)
		},
		// Error output (Issue #11 WS2): surface the FIRST real error line -
		// the payload, not the traceback header. The generic arm used to show
		// the first non-empty line, so a Python traceback previewed as
		// `Traceback (most recent call last):` and a compiler log as its
		// first `Compiling` line, both hiding the actual error. Never
		// success-looking: with no error line found, fall back to the last
		// non-empty line (tail = most recent state).
		"error" => {
			let hint = content
				.lines()
				.map(|l| l.trim())
				.find(|l| is_error_line(l) || is_failure_line(l))
				.or_else(|| content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
				.map(|l| l.chars().take(60).collect::<String>());
			match hint {
				Some(h) => format!("[error:{}L {}B | {}]", lines, bytes, h),
				None => format!("[error:{}L {}B]", lines, bytes),
			}
		},
		// Linter output (ruff/eslint/clippy/flake8): surface the first issue
		// line (`path:line:col: CODE message`, or `error:`/`warning:` lines).
		"linter" | "lint" => {
			let hint = content
				.lines()
				.map(|l| l.trim())
				.find(|l| is_lint_line(l))
				.or_else(|| content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
				.map(|l| l.chars().take(60).collect::<String>());
			match hint {
				Some(h) => format!("[lint:{}L {}B | {}]", lines, bytes, h),
				None => format!("[lint:{}L {}B]", lines, bytes),
			}
		},
		// Log output: the LAST non-empty line is the most recent state, and
		// an error/failure line (if any) is the signal that matters - prefer
		// it over the tail so a log ending in noise never hides the error.
		"log" => {
			let hint = content
				.lines()
				.map(|l| l.trim())
				.find(|l| is_error_line(l) || is_failure_line(l))
				.or_else(|| content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim()))
				.map(|l| l.chars().take(60).collect::<String>());
			match hint {
				Some(h) => format!("[log:{}L {}B | {}]", lines, bytes, h),
				None => format!("[log:{}L {}B]", lines, bytes),
			}
		},
		// Plain-text / unrecognized fallback: even when we can't classify the
		// shape, do better than a bare L/B count - show a content hint so the
		// agent has SOME signal about what the blob is.
		//
		// ISSUE-11 residuals #1/#4 (RC-D): the hint used to be the FIRST
		// non-empty line raw - a pretty-printed JSON blob previewed as a lone
		// `{` (MISLEADING, the top battery offender) and a 10 KB single-line
		// payload as a bare 60-char head (SHALLOW). Now: skip structural noise
		// lines (lone braces/brackets) to the first MEANINGFUL line (the first
		// key / statement), and sample head+tail for very long lines.
		_ => {
			let hint = first_meaningful_line(content)
				.map(|l| sample_long_line(&l))
				.filter(|s| !s.is_empty());
			match hint {
				Some(h) => format!("[{}:{}L {}B | {}]", type_str, lines, bytes, h),
				None => format!("[{}:{}L {}B]", type_str, lines, bytes),
			}
		},
	};
	// ── Issue #11 WS4: enforce the configured `preview_max_chars` cap ──
	apply_preview_cap(&preview, PREVIEW_MAX_CHARS.load(std::sync::atomic::Ordering::Relaxed) as usize)
}

/// git status preview: tally each two-char status code and list the first few
/// paths. `[git:5M 2A 1D 3?? | src/x.rs src/y.rs +6 more]`.
fn build_git_status_preview(content: &str, lines: usize) -> String {
	use std::collections::BTreeMap;
	let mut tally: BTreeMap<char, usize> = BTreeMap::new();
	let mut paths: Vec<String> = Vec::new();
	for line in content.lines() {
		if let Some(code) = git_status_code(line) {
			// Collapse the two columns to the most significant status char
			// (first non-space, non-`?` preferred, else the raw char).
			let ch = code.chars().find(|c| *c != ' ').unwrap_or('?');
			*tally.entry(ch).or_insert(0) += 1;
			if paths.len() < 3 {
				let p = line.get(3..).unwrap_or("").trim();
				if !p.is_empty() {
					// `R old -> new` renames: keep the new path.
					let p = p.rsplit(" -> ").next().unwrap_or(p);
					paths.push(p.chars().take(40).collect());
				}
			}
		}
	}
	if tally.is_empty() {
		return format!("[git:{}L]", lines);
	}
	// Emit tallies in a stable, readable order.
	let order = ['M', 'A', 'D', 'R', 'C', 'U', 'T', '?', '!'];
	let mut counts: Vec<String> = Vec::new();
	for c in order {
		if let Some(n) = tally.get(&c) {
			let label = if c == '?' { "??".to_string() } else { c.to_string() };
			counts.push(format!("{}{}", n, label));
		}
	}
	let total: usize = tally.values().sum();
	let shown = paths.len();
	let more = if total > shown { format!(" +{} more", total - shown) } else { String::new() };
	if paths.is_empty() {
		format!("[git:{}]", counts.join(" "))
	} else {
		format!("[git:{} | {}{}]", counts.join(" "), paths.join(" "), more)
	}
}

/// git log preview: commit count + first->last short hash and subject.
/// `[gitlog:20 commits | abc123 fix(x): … → def456 …]`.
fn build_gitlog_preview(content: &str, lines: usize) -> String {
	// Collect `commit <hash>` entries and, if present, the following subject.
	let all: Vec<&str> = content.lines().collect();
	let mut commits: Vec<(String, String)> = Vec::new();
	for (i, line) in all.iter().enumerate() {
		if let Some(rest) = line.strip_prefix("commit ") {
			let hash: String = rest.trim().chars().take(7).collect();
			// Subject: first non-empty, non-header line after the commit line.
			let subject = all[i + 1..]
				.iter()
				.map(|l| l.trim())
				.find(|l| {
					!l.is_empty()
						&& !l.starts_with("Author:")
						&& !l.starts_with("Date:")
						&& !l.starts_with("Merge:")
						&& !l.starts_with("commit ")
				})
				.unwrap_or("")
				.chars()
				.take(32)
				.collect::<String>();
			commits.push((hash, subject));
		}
	}
	if commits.is_empty() {
		return format!("[gitlog:{}L]", lines);
	}
	let n = commits.len();
	let first = &commits[0];
	if n == 1 {
		return format!("[gitlog:1 commit | {} {}]", first.0, first.1);
	}
	let last = &commits[n - 1];
	format!("[gitlog:{} commits | {} {} → {} {}]", n, first.0, first.1, last.0, last.1)
}

/// directory-listing preview: file/dir counts + top extensions.
/// `[ls:42 files 7 dirs | .rs×18 .md×9 …]`.
fn build_ls_preview(content: &str, lines: usize) -> String {
	use std::collections::HashMap;
	let mut files = 0usize;
	let mut dirs = 0usize;
	let mut ext: HashMap<String, usize> = HashMap::new();
	for line in content.lines() {
		let t = line.trim();
		if t.is_empty() {
			continue;
		}
		// Skip the `total N` header `ls -l` prints (not a filesystem entry).
		if let Some(rest) = t.strip_prefix("total ") {
			if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
				continue;
			}
		}
		let b = line.as_bytes();
		// `ls -l` long form: mode string in the first column.
		let is_long = b.len() >= 10
			&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
			&& b[1..10]
				.iter()
				.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'));
		let (is_dir, name) = if is_long {
			let name = line.split_whitespace().last().unwrap_or("");
			(b[0] == b'd', name)
		} else if let Some(stripped) = t.strip_suffix('/') {
			(true, stripped)
		} else {
			(false, t)
		};
		if is_dir {
			dirs += 1;
		} else {
			files += 1;
			// File extension: text after the last `.` in the basename.
			let base = name.rsplit('/').next().unwrap_or(name);
			if let Some(dot) = base.rfind('.') {
				if dot > 0 && dot < base.len() - 1 {
					let e: String = base[dot..].chars().take(8).collect();
					*ext.entry(e).or_insert(0) += 1;
				}
			}
		}
	}
	if files == 0 && dirs == 0 {
		return format!("[ls:{}L]", lines);
	}
	let mut top: Vec<(String, usize)> = ext.into_iter().collect();
	top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
	let ext_str = top
		.iter()
		.take(3)
		.map(|(e, n)| format!("{}×{}", e, n))
		.collect::<Vec<_>>()
		.join(" ");
	if ext_str.is_empty() {
		format!("[ls:{} files {} dirs]", files, dirs)
	} else {
		format!("[ls:{} files {} dirs | {}]", files, dirs, ext_str)
	}
}

/// test-output preview: pass/fail/ignored tallies + first failing test.
/// `[test:220 pass 0 fail 1 ignored | 0.31s]` / names the first failure.
fn build_test_preview(content: &str, lines: usize) -> String {
	// cargo: `test result: ok. 220 passed; 0 failed; 1 ignored; ... 0.31s`
	let mut pass = 0usize;
	let mut fail = 0usize;
	let mut ignored = 0usize;
	let mut found = false;
	for line in content.lines() {
		if let Some(rest) = line.split("test result:").nth(1) {
			found = true;
			pass += num_before(rest, "passed");
			fail += num_before(rest, "failed");
			ignored += num_before(rest, "ignored");
		}
	}
	// pytest: `=== 3 failed, 220 passed in 0.31s ===` / `220 passed`
	if !found {
		for line in content.lines() {
			if line.contains("passed") || line.contains("failed") {
				let p = num_before(line, "passed");
				let f = num_before(line, "failed");
				if p > 0 || f > 0 {
					found = true;
					pass += p;
					fail += f;
				}
			}
		}
	}
	// Bare test log without a summary line (residual #4, `term_plain`):
	// `test foo ... ok` / `... FAILED` / `... ignored` lines still give an
	// honest pass/fail tally instead of `[test:3L]`.
	if !found {
		for line in content.lines() {
			let t = line.trim();
			if let Some(rest) = t.strip_prefix("test ") {
				if rest.contains("... ok") {
					pass += 1;
					found = true;
				} else if rest.contains("... FAILED") {
					fail += 1;
					found = true;
				} else if rest.contains("... ignored") {
					ignored += 1;
					found = true;
				}
			}
		}
	}
	// First failing test name (cargo `test NAME ... FAILED` / go `--- FAIL: NAME`).
	let first_fail = content
		.lines()
		.find_map(|l| {
			let t = l.trim();
			if let Some(rest) = t.strip_prefix("--- FAIL: ") {
				Some(rest.split_whitespace().next().unwrap_or("").to_string())
			} else if t.starts_with("test ") && t.ends_with("FAILED") {
				t.strip_prefix("test ")
					.and_then(|r| r.split_whitespace().next())
					.map(|s| s.to_string())
			} else if t.starts_with("FAILED ") {
				t.strip_prefix("FAILED ")
					.map(|r| r.split_whitespace().next().unwrap_or("").to_string())
			} else {
				None
			}
		})
		.filter(|s| !s.is_empty());
	// Duration if present.
	let dur = content
		.lines()
		.find_map(|l| DUR_RE.captures(l).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()));

	if !found && first_fail.is_none() {
		return format!("[test:{}L]", lines);
	}
	let mut s = format!("[test:{} pass {} fail {} ignored", pass, fail, ignored);
	if let Some(f) = first_fail {
		s.push_str(&format!(" | FAIL {}", f.chars().take(40).collect::<String>()));
	} else if let Some(d) = dur {
		s.push_str(&format!(" | {}", d));
	}
	s.push(']');
	s
}

/// grep/ripgrep preview: hit count, distinct files, first location.
/// `[grep:38 hits in 9 files | src/x.rs:12 …]`.
fn build_grep_preview(content: &str, lines: usize) -> String {
	use std::collections::BTreeSet;
	let mut hits = 0usize;
	let mut files: BTreeSet<String> = BTreeSet::new();
	let mut first: Option<String> = None;
	for line in content.lines() {
		if !is_grep_line(line) {
			continue;
		}
		hits += 1;
		let mut it = line.splitn(3, ':');
		let path = it.next().unwrap_or("");
		let lno = it.next().unwrap_or("");
		files.insert(path.to_string());
		if first.is_none() {
			first = Some(format!("{}:{}", path, lno).chars().take(48).collect());
		}
	}
	if hits == 0 {
		return format!("[grep:{}L]", lines);
	}
	match first {
		Some(loc) => format!("[grep:{} hits in {} files | {} …]", hits, files.len(), loc),
		None => format!("[grep:{} hits in {} files]", hits, files.len()),
	}
}

/// Parse the integer immediately preceding `keyword` on a line (e.g. `220
/// passed` -> 220). Returns 0 when absent.
fn num_before(line: &str, keyword: &str) -> usize {
	let idx = match line.find(keyword) {
		Some(i) => i,
		None => return 0,
	};
	line[..idx]
		.trim_end()
		.rsplit(|c: char| !c.is_ascii_digit())
		.find(|s| !s.is_empty())
		.and_then(|s| s.parse().ok())
		.unwrap_or(0)
}

static DUR_RE: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"(\d+\.\d+s|\d+ms)").unwrap());

/// JSON preview: parse content and show item/object count with top-level keys,
/// matching the quality of stage2's `reduce_json`. Falls back to a crude `{"`
/// count when parsing fails (e.g. truncated or malformed JSON).
fn build_json_preview(content: &str, lines: usize) -> String {
	match serde_json::from_str::<JsonValue>(content) {
		Ok(JsonValue::Array(arr)) => {
			let keys = arr
				.first()
				.and_then(|v| v.as_object())
				.map(|obj| {
					let ks: Vec<&str> = obj.keys().map(|k| k.as_str()).take(8).collect();
					let more = if ks.len() < obj.len() {
						format!(" +{} more", obj.len() - ks.len())
					} else {
						String::new()
					};
					format!(" | keys: {}{}", ks.join(", "), more)
				})
				.unwrap_or_default();
			format!("[json:{}items {}L{}]", arr.len(), lines, keys)
		},
		Ok(JsonValue::Object(obj)) => {
			let ks: Vec<&str> = obj.keys().map(|k| k.as_str()).take(8).collect();
			let more = if ks.len() < obj.len() {
				format!(" +{} more", obj.len() - ks.len())
			} else {
				String::new()
			};
			format!("[json:{}keys {}L | {}{}]", obj.len(), lines, ks.join(", "), more)
		},
		_ => {
			// Fallback: crude `{"` count for unparseable content.
			let i = content.matches("{\"").count();
			format!("[json:~{}items {}L]", i, lines)
		},
	}
}

/// Search preview: grep/ripgrep hit count, distinct files, first match location.
/// Uses the same regex pattern as `content_detector::SEARCH_RESULT_PATTERN`
/// (`file:line:` format).
fn build_search_preview(content: &str, lines: usize) -> String {
	use std::collections::BTreeSet;
	static SEARCH_RE: std::sync::LazyLock<regex::Regex> =
		std::sync::LazyLock::new(|| regex::Regex::new(r"^[^\s:]+:\d+:").unwrap());
	let mut hits = 0usize;
	let mut files: BTreeSet<String> = BTreeSet::new();
	let mut first: Option<String> = None;
	for line in content.lines() {
		if line.trim().is_empty() {
			continue;
		}
		if !SEARCH_RE.is_match(line) {
			continue;
		}
		hits += 1;
		if let Some((path, rest)) = line.split_once(':') {
			files.insert(path.to_string());
			if first.is_none() {
				let lno = rest.split(':').next().unwrap_or("");
				first = Some(format!("{}:{}", path, lno).chars().take(48).collect());
			}
		}
	}
	if hits == 0 {
		return format!("[search:{}L]", lines);
	}
	match first {
		Some(loc) => format!("[search:{} hits in {} files | {} …]", hits, files.len(), loc),
		None => format!("[search:{} hits in {} files]", hits, files.len()),
	}
}

/// Markdown table preview: column count, row count, header cells.
/// `[table:3 cols 4 rows | Name, Age, City]`. Rows = non-empty lines minus
/// separator rows (header + data rows).
fn build_table_preview(content: &str, lines: usize) -> String {
	let is_sep = |l: &str| {
		let cells = l.trim().trim_matches('|');
		!cells.is_empty()
			&& cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
			})
	};
	let rows: Vec<&str> = content
		.lines()
		.map(|l| l.trim())
		.filter(|l| !l.is_empty() && !is_sep(l))
		.collect();
	let header = rows.first().unwrap_or(&"").trim_matches('|');
	let cells: Vec<&str> = header.split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
	let cols = cells.len();
	let shown: Vec<&str> = cells.iter().take(5).copied().collect();
	let more = if cols > shown.len() { format!(" +{} more", cols - shown.len()) } else { String::new() };
	format!("[table:{} cols {} rows | {}{}]", cols, rows.len(), shown.join(", "), more)
}

/// Markdown document preview: heading tally + first heading.
/// `[md:7L h1×1 h2×2 | # Release Notes]`.
fn build_markdown_preview(content: &str, lines: usize) -> String {
	let mut levels: Vec<usize> = Vec::new();
	let mut first_heading: Option<String> = None;
	for line in content.lines() {
		if is_md_heading(line) {
			let t = line.trim_start();
			let n = t.chars().take_while(|c| *c == '#').count();
			levels.push(n);
			if first_heading.is_none() {
				first_heading = Some(t.chars().take(48).collect());
			}
		}
	}
	if levels.is_empty() {
		return format!("[md:{}L]", lines);
	}
	let tally: Vec<String> = (1..=6)
		.filter_map(|l| {
			let c = levels.iter().filter(|&&x| x == l).count();
			if c > 0 {
				Some(format!("h{l}×{c}"))
			} else {
				None
			}
		})
		.collect();
	match first_heading {
		Some(h) => format!("[md:{}L {} | {}]", lines, tally.join(" "), h),
		None => format!("[md:{}L {}]", lines, tally.join(" ")),
	}
}

/// YAML preview: top-level key count + first few keys.
/// `[yaml:4 keys 6L | name, version, port, debug]`.
fn build_yaml_preview(content: &str, lines: usize) -> String {
	let keys: Vec<&str> = content
		.lines()
		.filter(|l| is_yaml_key_line(l))
		.map(|l| l[..l.find(':').unwrap_or(0)].to_string())
		.collect();
	let shown: Vec<&str> = keys.iter().take(5).map(|s| s.as_str()).collect();
	let more = if keys.len() > shown.len() {
		format!(" +{} more", keys.len() - shown.len())
	} else {
		String::new()
	};
	format!("[yaml:{} keys {}L | {}{}]", keys.len(), lines, shown.join(", "), more)
}

/// XML preview: element count + root tag. `[xml:3 elements 4L | <root>]`.
fn build_xml_preview(content: &str, lines: usize) -> String {
	let elements = content.matches("</").count();
	let root = content
		.lines()
		.map(|l| l.trim())
		.find(|l| l.starts_with('<'))
		.map(|l| {
			let tag = l[1..]
				.split(|c: char| c.is_whitespace() || c == '>' || c == '/')
				.next()
				.unwrap_or("");
			format!("<{}>", tag)
		});
	match root {
		Some(r) => format!("[xml:{} elements {}L | {}]", elements, lines, r),
		None => format!("[xml:{} elements {}L]", elements, lines),
	}
}

/// CSV preview: row count, column count, header cells.
/// `[csv:4 rows 3 cols | name, age, city]`.
fn build_csv_preview(content: &str, lines: usize) -> String {
	let rows: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
	let cols = rows.first().map(|r| r.split(',').count()).unwrap_or(0);
	let header: Vec<&str> = rows
		.first()
		.map(|r| r.split(',').map(|c| c.trim()).take(5).collect())
		.unwrap_or_default();
	let more = if cols > header.len() {
		format!(" +{} more", cols - header.len())
	} else {
		String::new()
	};
	format!("[csv:{} rows {} cols | {}{}]", rows.len(), cols, header.join(", "), more)
}

/// HTML preview: title, heading count, link count, body size estimate.
fn build_html_preview(content: &str, lines: usize) -> String {
	// Extract <title>…</title> text (anywhere on a line, case-insensitive).
	let title = content.lines().find_map(|l| {
		let lower = l.to_lowercase();
		let start = lower.find("<title>")? + 7;
		let end = lower[start..].find("</title>")?;
		Some(l[start..start + end].trim().chars().take(60).collect::<String>())
	});
	// Count common structural elements.
	let headings = content.matches("<h1").count()
		+ content.matches("<h2").count()
		+ content.matches("<h3").count()
		+ content.matches("<H1").count()
		+ content.matches("<H2").count()
		+ content.matches("<H3").count();
	let links = content.matches("<a ").count() + content.matches("<A ").count();
	let imgs = content.matches("<img ").count() + content.matches("<IMG ").count();
	let scripts = content.matches("<script").count() + content.matches("<SCRIPT").count();

	let mut parts: Vec<String> = Vec::new();
	if let Some(t) = title {
		parts.push(t);
	}
	let mut stats: Vec<String> = Vec::new();
	if headings > 0 {
		stats.push(format!("{}h", headings));
	}
	if links > 0 {
		stats.push(format!("{}a", links));
	}
	if imgs > 0 {
		stats.push(format!("{}img", imgs));
	}
	if scripts > 0 {
		stats.push(format!("{}script", scripts));
	}
	stats.push(format!("{}L", lines));
	let stats_str = stats.join(" ");
	if parts.is_empty() {
		format!("[html:{}]", stats_str)
	} else {
		format!("[html:{} | {}]", stats_str, parts.join(" | "))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// ── 04-T9: pathological-input coverage (UTF-8 boundary, literal
	// markers, interior NUL) - previously untested in this module. ──

	#[test]
	fn test_detect_type_never_panics_on_interior_nul() {
		let _g = cap_guard();
		let content = "before\0after\0\0end";
		let _ = detect_type(content); // must not panic
	}

	#[test]
	fn test_build_preview_never_panics_on_interior_nul_across_type_branches() {
		let _g = cap_guard();
		let content = "line one\0line two\0\0error: boom\nwarning: also this";
		for ty in [
			"build",
			"diff",
			"code_rust",
			"code_sh",
			"code",
			"search",
			"html",
			"json_array",
			"json",
			"json_list",
			"terminal",
			"text",
		] {
			let _ = build_preview(ty, content); // must not panic for any branch
		}
	}

	#[test]
	fn test_build_preview_code_rust_truncates_signature_on_char_boundary() {
		let _g = cap_guard();
		// The "code" family truncates the first signature to 48 *chars* via
		// `.chars().take(48)`, not a byte slice - a signature packed with
		// multi-byte UTF-8 must not panic or split a character mid-encoding.
		// This function never panics on reaching an assertion at all (a byte-
		// boundary split would have panicked inside `String` construction
		// before we got here), so the pass condition is simply completing
		// without a panic and producing the expected preview shape.
		let content = format!("fn \u{4e2d}\u{6587}_{}() {{}}", "x".repeat(60));
		let out = build_preview("code_rust", &content);
		assert!(out.starts_with("[code:"));
	}

	#[test]
	fn test_build_preview_never_panics_on_multibyte_utf8_every_type() {
		let _g = cap_guard();
		let content = "a\u{00e9}\u{4e2d}\u{1f600}b".repeat(30);
		for ty in [
			"build",
			"diff",
			"code_rust",
			"code_sh",
			"code",
			"search",
			"html",
			"json_array",
			"json",
			"json_list",
			"terminal",
			"text",
		] {
			let _ = build_preview(ty, &content);
		}
	}

	#[test]
	fn test_build_preview_handles_literal_marker_shaped_content() {
		let _g = cap_guard();
		// Content that already contains marker-shaped text (e.g. a pasted
		// example transcript) must not confuse the line/byte counting or
		// panic in any branch - build_preview only ever summarizes, it never
		// re-parses content as a marker.
		let content = "before <<<CCR:fake000|text|1>>> after\nerror: boom";
		for ty in [
			"build",
			"diff",
			"code_rust",
			"code_sh",
			"code",
			"search",
			"html",
			"json_array",
			"json",
			"json_list",
			"terminal",
			"text",
		] {
			let out = build_preview(ty, content);
			assert!(!out.is_empty());
		}
	}

	#[test]
	fn test_detect_type_never_panics_on_multibyte_utf8() {
		let _g = cap_guard();
		let content = "\u{1f600}".repeat(500);
		let _ = detect_type(&content);
	}

	// ── Enriched preview arms (report 09 §5): realistic fixtures, exact
	// enriched output. Each preview must be self-describing `[type:...]` and
	// pack decision-relevant facts. ──

	const GIT_STATUS: &str = " M crates/aphrodite/src/preview.rs\n M crates/aphrodite/src/hooks.rs\nA  src/new_a.rs\nA  src/new_b.rs\nD  \
		 src/old.rs\n?? tmp/scratch\n?? tmp/other\n?? build/log";

	#[test]
	fn test_detect_and_preview_git_status() {
		let _g = cap_guard();
		assert_eq!(detect_semantic_type(GIT_STATUS), Some("git"));
		let p = build_preview("text", GIT_STATUS);
		// 2 M, 2 A, 1 D, 3 ?? + first 3 paths + "+5 more".
		assert_eq!(
			p,
			"[git:2M 2A 1D 3?? | crates/aphrodite/src/preview.rs crates/aphrodite/src/hooks.rs src/new_a.rs +5 more]"
		);
	}

	#[test]
	fn test_preview_git_status_rename() {
		let _g = cap_guard();
		let c = "R  old/path.rs -> new/path.rs\nR  a.txt -> b.txt";
		let p = build_preview("git", c);
		assert!(p.starts_with("[git:2R | new/path.rs b.txt"), "got {p}");
	}

	const CARGO_TEST: &str = "running 221 tests\ntest foo::bar ... ok\ntest result: ok. 220 passed; 0 failed; 1 \
	                         ignored; 0 measured; 0 filtered out; finished in 0.31s";

	#[test]
	fn test_detect_and_preview_cargo_test() {
		let _g = cap_guard();
		assert_eq!(detect_semantic_type(CARGO_TEST), Some("test"));
		let p = build_preview("text", CARGO_TEST);
		assert_eq!(p, "[test:220 pass 0 fail 1 ignored | 0.31s]");
	}

	#[test]
	fn test_preview_test_names_first_failure() {
		let _g = cap_guard();
		let c = "running 3 tests\ntest alpha ... ok\ntest beta ... FAILED\ntest result: FAILED. 2 passed; 1 failed; 0 \
		         ignored; finished in 0.05s";
		let p = build_preview("text", c);
		assert!(p.starts_with("[test:2 pass 1 fail 0 ignored | FAIL beta"), "got {p}");
	}

	const LS_LONG: &str = "total 48\ndrwxr-xr-x  5 nikola staff  160 Jul 14 10:00 src\ndrwxr-xr-x  2 nikola staff   64 \
	                      Jul 14 10:00 tests\n-rw-r--r--  1 nikola staff 1913 Jul 14 10:00 preview.rs\n-rw-r--r--  1 \
	                      nikola staff  820 Jul 14 10:00 hooks.rs\n-rw-r--r--  1 nikola staff  512 Jul 14 10:00 \
	                      README.md";

	#[test]
	fn test_detect_and_preview_ls_long() {
		let _g = cap_guard();
		assert_eq!(detect_semantic_type(LS_LONG), Some("ls"));
		let p = build_preview("text", LS_LONG);
		// 3 files, 2 dirs; extensions .rs×2 .md×1 (the `total 48` line is skipped).
		assert_eq!(p, "[ls:3 files 2 dirs | .rs×2 .md×1]");
	}

	const RIPGREP: &str = "src/preview.rs:12:    let lines = content.lines().count();\nsrc/preview.rs:88:    \
	                      format!(\"[terminal...\nsrc/hooks.rs:91:    let preview = \
	                      crate::build_preview();\nsrc/marker.rs:49:    let mut safe = preview.replace();";

	#[test]
	fn test_detect_and_preview_ripgrep() {
		let _g = cap_guard();
		assert_eq!(detect_semantic_type(RIPGREP), Some("grep"));
		let p = build_preview("text", RIPGREP);
		assert_eq!(p, "[grep:4 hits in 3 files | src/preview.rs:12 …]");
	}

	const GIT_LOG: &str = "commit abc1234def5678\nAuthor: Nikola <n@x.io>\nDate:   Mon Jul 14\n\n    fix(preview): \
	                      stop doubling\n\ncommit def5678abc1234\nAuthor: Nikola <n@x.io>\nDate:   Sun Jul 13\n\n    \
	                      feat: add detector";

	#[test]
	fn test_detect_and_preview_git_log() {
		let _g = cap_guard();
		assert_eq!(detect_semantic_type(GIT_LOG), Some("gitlog"));
		let p = build_preview("text", GIT_LOG);
		assert_eq!(
			p,
			"[gitlog:2 commits | abc1234 fix(preview): stop doubling → def5678 feat: add detector]"
		);
	}

	#[test]
	fn test_preview_build_surfaces_first_error() {
		let _g = cap_guard();
		let c = "   Compiling aphrodite v1.3.3\nerror[E0432]: unresolved import `crate::foo`\n  --> \
		         src/x.rs:1:5\nwarning: unused variable `y`";
		let p = build_preview("build", c);
		assert!(p.starts_with("[build:"), "got {p}");
		assert!(p.contains("E0432]: unresolved import"), "must surface first error text: {p}");
	}

	#[test]
	fn test_preview_diff_names_files() {
		let _g = cap_guard();
		let c = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,2 +1,3 @@\n+new\ndiff --git a/Cargo.toml \
		         b/Cargo.toml\n@@ -1 +1 @@\n-x\n+y";
		let p = build_preview("diff", c);
		assert!(p.contains("src/main.rs"), "got {p}");
		assert!(p.contains("Cargo.toml"), "got {p}");
	}

	#[test]
	fn test_fallback_shows_first_line_hint() {
		let _g = cap_guard();
		let c = "some unrecognizable prose here\nline two\nline three";
		let p = build_preview("text", c);
		// No semantic shape detected -> generic fallback WITH a first-line hint.
		assert_eq!(p, "[text:3L 50B | some unrecognizable prose here]");
	}

	#[test]
	fn test_semantic_detector_leaves_prose_alone() {
		let _g = cap_guard();
		let prose = "The quick brown fox jumps over the lazy dog.\nAnother sentence of ordinary prose.";
		assert_eq!(detect_semantic_type(prose), None);
	}

	// ── JSON preview (intelligent parsing, not crude `{"` count) ──

	#[test]
	fn test_json_preview_array_shows_keys() {
		let _g = cap_guard();
		let c = "[{\"status\":\"ok\",\"count\":42},{\"status\":\"err\",\"count\":0}]";
		let p = build_preview("json_array", c);
		assert_eq!(p, "[json:2items 1L | keys: status, count]");
	}

	#[test]
	fn test_json_preview_object_shows_keys() {
		let _g = cap_guard();
		let c = "{\"status\":\"in_progress\",\"conclusion\":null,\"jobs\":[{\"name\":\"Test\"}]}";
		let p = build_preview("json", c);
		assert!(p.starts_with("[json:3keys"));
		assert!(p.contains("status"));
		assert!(p.contains("conclusion"));
		assert!(p.contains("jobs"));
	}

	#[test]
	fn test_json_preview_fallback_on_unparseable() {
		let _g = cap_guard();
		let c = "not valid json {at all";
		let p = build_preview("json_array", c);
		assert!(p.starts_with("[json:~"));
	}

	// ── Search preview (proper regex, not bare `:` count) ──

	#[test]
	fn test_search_preview_counts_hits_and_files() {
		let _g = cap_guard();
		let c = "src/main.rs:12:    let x = 1;\nsrc/main.rs:42:    println!();\nsrc/lib.rs:7:    pub fn foo()";
		let p = build_preview("search", c);
		assert!(p.starts_with("[search:3 hits in 2 files | "), "got {p}");
	}

	#[test]
	fn test_search_preview_ignores_non_search_lines() {
		let _g = cap_guard();
		let c = "src/main.rs:12:    let x = 1;\njust some prose here\nsrc/lib.rs:7:    pub fn foo()";
		let p = build_preview("search", c);
		assert!(p.contains("2 hits"), "got {p}");
	}

	// ── HTML preview (title, headings, links, images) ──

	#[test]
	fn test_html_preview_extracts_title_and_counts() {
		let _g = cap_guard();
		let c = "<!DOCTYPE html>\n<html>\n<head><title>My Page</title></head>\n<body>\n<h1>Hello</h1>\n<h2>Section</h2>\n<a href=\"/x\">link</a>\n<a href=\"/y\">link2</a>\n<img src=\"a.png\">\n</body>\n</html>";
		let p = build_preview("html", c);
		assert!(p.contains("My Page"), "must include title, got {p}");
		assert!(p.contains("2h"), "got {p}");
		assert!(p.contains("2a"), "got {p}");
		assert!(p.contains("1img"), "got {p}");
	}

	#[test]
	fn test_new_arms_never_panic_on_pathological_input() {
		let multibyte = "a\u{00e9}\u{4e2d}\u{1f600}".repeat(20);
		let inputs = ["", "\0\0\0", multibyte.as_str()];
		for ty in ["git", "gitlog", "ls", "test", "grep"] {
			for c in inputs {
				let _ = build_preview(ty, c);
				let _ = detect_semantic_type(c);
			}
		}
	}

	// ── Issue #11 WS2: honest build arm (real counts, no fabricated
	// success) ──

	#[test]
	fn test_build_preview_counts_error_warning_lines_not_substrings() {
		let _g = cap_guard();
		// One line with TWO `error` occurrences, a capitalized `Error:` line
		// (invisible to the old lowercase substring counter), an unrelated
		// `error`-shaped word, and a warning line: the OLD counter reported
		// 3E (occurrences) and missed the capitalized one entirely; the honest
		// counter reports 2 error lines + 1 warning line.
		let c = "error[E0432]: error in crate foo\nError: failed to build\nwarning: unused import\nnoerror here";
		let p = build_preview("build", c);
		assert!(p.starts_with("[build:2E 1W 4L"), "line-based counts expected, got {p}");
	}

	#[test]
	fn test_build_preview_failing_test_never_looks_clean() {
		let _g = cap_guard();
		// ISSUE-11-PREVIEW-BATTERY #2: `test result: FAILED. 3 failed` used
		// to preview as `[build:0E 0W ...]` - a failing test run looked like
		// a clean build. The failure signal must surface.
		let c = "running 3 tests\ntest alpha ... ok\ntest result: FAILED. 1 passed; 2 failed; finished in 0.05s";
		let p = build_preview("build_output", c);
		assert!(p.starts_with("[build:0E 0W 3L | "), "failure note expected, got {p}");
		assert!(p.contains("FAILED"), "failure summary must be visible: {p}");
		assert!(!p.starts_with("[build:0E 0W 3L]"), "clean-looking summary is the bug: {p}");
	}

	#[test]
	fn test_build_preview_passing_test_keeps_clean_summary() {
		let _g = cap_guard();
		// ISSUE-11 residual #4: a PASSING test run the classifier tagged
		// `build_output` must not render as a clean-looking `[build:0E 0W 3L]`
		// (SHALLOW - hides that the payload is a test run). It upgrades to the
		// test arm with the real tally; `0 fail` keeps the "not flagged as
		// failure" intent of the original pin.
		let c = "running 3 tests\ntest alpha ... ok\ntest result: ok. 3 passed; 0 failed; finished in 0.05s";
		let p = build_preview("build_output", c);
		assert_eq!(p, "[test:3 pass 0 fail 0 ignored | 0.05s]");
	}

	#[test]
	fn test_build_preview_surfaces_capitalized_error_line() {
		let _g = cap_guard();
		// `Error:` (Python/Swift/clang) was invisible to the old lowercase
		// substring count; it must now count AND surface as the first error.
		let c = "   Compiling foo v0.1.0\nError: failed to run custom build command";
		let p = build_preview("build", c);
		assert!(p.starts_with("[build:1E 0W 2L | Error: failed to run"), "got {p}");
	}

	// ── Issue #11 WS2: honest error/linter/log arms (never success-looking,
	// never a useless first line) ──

	#[test]
	fn test_preview_error_surfaces_the_error_line_not_traceback_header() {
		let _g = cap_guard();
		let c = "Traceback (most recent call last):\n  File \"x.py\", line 3, in <module>\nValueError: disk full";
		let p = build_preview("error", c);
		assert!(p.starts_with("[error:3L"), "got {p}");
		assert!(p.contains("ValueError: disk full"), "error arm must surface the real error: {p}");
		assert!(!p.contains("Traceback"), "traceback header is not the error: {p}");
	}

	#[test]
	fn test_preview_lint_surfaces_first_issue_line() {
		let _g = cap_guard();
		let c = "src/x.py:10:5: E501 line too long (98 > 88)\nsrc/x.py:12:1: W0611 unused import os";
		let p = build_preview("lint", c);
		assert!(p.starts_with("[lint:2L"), "got {p}");
		assert!(p.contains("E501"), "linter arm must surface the issue: {p}");
	}

	#[test]
	fn test_preview_log_surfaces_error_signal_or_tail() {
		let _g = cap_guard();
		// Error signal wins over the tail...
		let with_err = "INFO starting\nWARN retry\nERROR connection refused\nINFO gave up";
		let p = build_preview("log", with_err);
		assert!(p.contains("connection refused"), "log arm must surface the error line: {p}");
		// ...otherwise the last non-empty line (most recent state).
		let tail = "2026-09-17T10:00:00Z INFO start\n2026-09-17T10:00:05Z INFO done";
		let p2 = build_preview("log", tail);
		assert!(p2.contains("INFO done"), "log arm must surface the tail line: {p2}");
	}

	// ── Issue #11 WS4: preview_max_chars cap end-to-end ──

	/// Serializes tests that mutate the process-global preview cap (cargo
	/// runs this module's tests concurrently; the cap is process-wide).
	fn cap_guard() -> std::sync::MutexGuard<'static, ()> {
		crate::preview::preview_cap_test_guard()
	}

	#[test]
	fn test_preview_cap_truncates_and_keeps_bracket() {
		let _g = cap_guard();
		let prev = set_preview_max_chars(Some(30));
		let long = format!("some {} prose", "x".repeat(200));
		let p = build_preview("text", &long);
		assert!(p.chars().count() <= 30, "preview must respect the cap: {p}");
		assert!(p.ends_with(']'), "self-bracketed preview must keep its closing bracket: {p}");
		assert_eq!(p.chars().count(), 30, "truncated preview should fill the budget exactly: {p}");
		set_preview_max_chars(prev);
	}

	#[test]
	fn test_preview_cap_unset_is_unlimited() {
		let _g = cap_guard();
		set_preview_max_chars(None);
		let long = format!("some {} prose", "y".repeat(200));
		let p = build_preview("text", &long);
		assert!(p.chars().count() > 30, "no cap -> full preview expected: {p}");
		assert!(p.contains("yyy"), "got {p}");
	}

	#[test]
	fn test_preview_cap_multibyte_truncates_on_char_boundary() {
		let _g = cap_guard();
		let prev = set_preview_max_chars(Some(25));
		let content = format!("{}{}", "a\u{00e9}\u{4e2d}\u{1f600}".repeat(30), " end");
		let p = build_preview("text", &content);
		assert!(p.chars().count() <= 25, "multibyte truncation must stay on a char boundary: {p}");
		set_preview_max_chars(prev);
	}

	#[test]
	fn test_preview_cap_applies_to_build_arm_too() {
		let _g = cap_guard();
		let prev = set_preview_max_chars(Some(20));
		let c = "error[E0432]: unresolved import `crate::foo`\n  --> src/x.rs:1:5\nwarning: unused variable `y`";
		let p = build_preview("build", c);
		assert!(p.chars().count() <= 20, "cap must apply to every arm: {p}");
		assert!(p.ends_with(']'));
		set_preview_max_chars(prev);
	}

	// ── ISSUE-11 residuals #1/#2/#4 (the honest tail) ──────────────

	// #1: pretty-printed JSON with a `tool_result` hint previewed as a lone
	// `{` (MISLEADING, the top battery offender). The generic arm must skip
	// the structural brace line and show the first MEANINGFUL line.
	#[test]
	fn test_preview_pretty_json_lone_brace_never_previews_as_brace() {
		let _g = cap_guard();
		// Wrapper-envelope keys (name/version) keep the raw-JSON preview; the
		// generic arm must NOT show the lone `{` opener.
		let c = "{\n  \"name\": \"webapp\",\n  \"version\": \"1.0.0\",\n  \"port\": 8080\n}";
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[tool_result:5L"), "got {p}");
		assert!(
			p.contains("\"name\": \"webapp\""),
			"first meaningful line (first key) must be shown, got {p}"
		);
		assert!(!p.contains("| {]"), "lone brace preview is the bug: {p}");
	}

	// #1: pretty-printed JSON WITHOUT wrapper-envelope keys routes to the JSON
	// arm (detection upgrade RC-C) - keys + counts, never a brace.
	#[test]
	fn test_preview_pretty_json_routes_to_json_arm() {
		let _g = cap_guard();
		let c = "{\n  \"web\": [\n    { \"title\": \"x\" }\n  ]\n}";
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[json:"), "pretty JSON must route to the json arm, got {p}");
		assert!(p.contains("web"), "top-level keys must be listed: {p}");
	}

	// #1/#4: a single-line JSON object with no hint (nested_obj_nohint /
	// flat_json_nohint battery rows) used to preview as `[text:...]` SHALLOW;
	// detection now routes it to the json arm.
	#[test]
	fn test_preview_nested_json_object_routes_to_json_arm() {
		let _g = cap_guard();
		let c = "{\"data\": {\"user\": {\"name\": \"Alice\", \"email\": \"a@b.c\"}}}";
		assert_eq!(detect_semantic_type(c), Some("json"));
		let p = build_preview("text", c);
		assert!(p.starts_with("[json:1keys 1L | data]"), "got {p}");
	}

	// #2: terminal arm shows the FIRST meaningful line, never the last (the
	// `[terminal:7L }]` bug class). An exit-code line still wins.
	#[test]
	fn test_terminal_arm_shows_first_meaningful_line_not_last() {
		let _g = cap_guard();
		let c = "$ run script\noutput line\n}";
		let p = build_preview("terminal", c);
		assert!(p.starts_with("[terminal:3L $ run script]"), "got {p}");
		assert!(!p.contains("| }]"), "closing-brace preview is the bug: {p}");
	}

	// #2: rust code with a `terminal` hint (rust_code_hint_terminal battery
	// row) - detection upgrades it to the code arm instead of `[terminal:7L }]`.
	#[test]
	fn test_terminal_hint_with_code_routes_to_code_arm() {
		let _g = cap_guard();
		let c = "use std::collections::HashMap;\n\nfn main() {\n    let mut map = HashMap::new();\n    map.insert(\"a\", 1);\n    println!(\"{:?}\", map);\n}";
		assert_eq!(detect_semantic_type(c), Some("code"));
		let p = build_preview("terminal", c);
		assert!(p.starts_with("[code:"), "code with terminal hint must get the code arm, got {p}");
		assert!(p.contains("fn main"), "first signature must be visible: {p}");
	}

	// #4 (RC-C): raw diff with a `tool_result` hint (diff_raw battery row)
	// upgrades to the diff arm with the changed file named.
	#[test]
	fn test_detect_diff_raw_upgrades_to_diff_arm() {
		let _g = cap_guard();
		let c = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,3 @@\n-old line\n+new line\ncontext";
		assert_eq!(detect_semantic_type(c), Some("diff"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[diff:"), "raw diff must get the diff arm, got {p}");
		assert!(p.contains("foo.rs"), "changed file must be named: {p}");
	}

	// #4 (RC-C): yaml with a `tool_result` hint upgrades to the yaml arm.
	#[test]
	fn test_detect_yaml_upgrades_to_yaml_arm() {
		let _g = cap_guard();
		let c = "name: webapp\nversion: 1.0.0\nport: 8080\ndebug: true";
		assert_eq!(detect_semantic_type(c), Some("yaml"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[yaml:4 keys 4L | name, version, port, debug]"), "got {p}");
	}

	// #4 (RC-C): markdown table upgrades to the table arm with cols+rows+header.
	#[test]
	fn test_detect_markdown_table_upgrades_to_table_arm() {
		let _g = cap_guard();
		let c = "| Name | Age | City |\n|------|-----|------|\n| Alice | 30 | NYC |\n| Bob | 25 | LA |";
		assert_eq!(detect_semantic_type(c), Some("table"));
		let p = build_preview("tool_result", c);
		assert!(
			p.starts_with("[table:3 cols 3 rows | Name, Age, City]"),
			"got {p}"
		);
	}

	// #4 (RC-C): csv, xml, markdown doc, build log all upgrade off the generic
	// first-line arm.
	#[test]
	fn test_detect_csv_upgrades_to_csv_arm() {
		let _g = cap_guard();
		let c = "name,age,city\nalice,30,nyc\nbob,25,la\ncarol,28,sf";
		assert_eq!(detect_semantic_type(c), Some("csv"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[csv:4 rows 3 cols | name, age, city]"), "got {p}");
	}

	#[test]
	fn test_detect_xml_upgrades_to_xml_arm() {
		let _g = cap_guard();
		let c = "<root>\n  <item>one</item>\n  <item>two</item>\n</root>";
		assert_eq!(detect_semantic_type(c), Some("xml"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[xml:3 elements 4L | <root>]"), "got {p}");
	}

	#[test]
	fn test_detect_markdown_doc_upgrades_to_markdown_arm() {
		let _g = cap_guard();
		let c = "# Release Notes\n\n## Features\n- new previews\n- honest counts\n\n## Fixes";
		assert_eq!(detect_semantic_type(c), Some("markdown"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[md:"), "got {p}");
		assert!(p.contains("h1×1 h2×2"), "heading tally expected: {p}");
		assert!(p.contains("# Release Notes"), "first heading expected: {p}");
	}

	#[test]
	fn test_detect_build_log_upgrades_to_build_arm() {
		let _g = cap_guard();
		let c = "Compiling foo v0.1.0\nCompiling bar v0.1.1\nFinished dev [unoptimized] target(s) in 0.42s";
		assert_eq!(detect_semantic_type(c), Some("build"));
		let p = build_preview("tool_result", c);
		assert_eq!(p, "[build:0E 0W 3L]");
	}

	// #4: git status / ls with a `tool_result` hint (git_status_raw / ls_raw
	// battery rows) - the semantic detectors already knew these shapes; the
	// hint made them unreachable. The upgrade set now includes `tool_result`.
	#[test]
	fn test_tool_result_hint_still_upgrades_git_and_ls() {
		let _g = cap_guard();
		let git = "M src/a.rs\n?? tmp/scratch\nA  src/new.rs";
		let p = build_preview("tool_result", git);
		assert!(p.starts_with("[git:"), "git status with tool_result hint: {p}");
		let p2 = build_preview("tool_result", LS_LONG);
		assert!(p2.starts_with("[ls:"), "ls listing with tool_result hint: {p2}");
	}

	// #4 (RC-D): a very long single line (long_single_line / repeated battery
	// rows) samples head+tail instead of a bare head.
	#[test]
	fn test_generic_arm_long_line_samples_head_and_tail() {
		let _g = cap_guard();
		let c = format!("{}", "word ".repeat(2000));
		let p = build_preview("tool_result", &c);
		assert!(p.starts_with("[tool_result:1L 10000B | "), "got {p}");
		assert!(p.contains("…"), "long line must be sampled head+tail: {p}");
		assert!(p.contains("word"), "both ends are words: {p}");
		assert!(p.chars().count() < 120, "sample must stay compact: {p}");
	}

	// #4: a bare test log without a summary line (term_plain battery row)
	// upgrades to the test arm with an honest pass/fail tally.
	#[test]
	fn test_bare_test_log_without_summary_gets_test_arm() {
		let _g = cap_guard();
		let c = "running 3 tests\ntest alpha ... ok\ntest beta ... ok";
		assert_eq!(detect_semantic_type(c), Some("test"));
		let p = build_preview("tool_result", c);
		assert!(p.starts_with("[test:2 pass 0 fail 0 ignored]"), "got {p}");
	}

	// #4: wrapper-envelope JSON (success/output/diff keys) must NOT be hijacked
	// into the json arm - the raw wrapper preview is the payload (WS1).
	#[test]
	fn test_envelope_json_objects_are_not_hijacked_to_json_arm() {
		let _g = cap_guard();
		for c in [
			"{\"success\": true}",
			"{\"output\": \"running 10 tests\\n\\nte\", \"exit_code\": 0}",
			"{\"error\": \"file not found: foo.rs\"}",
		] {
			assert_ne!(detect_semantic_type(c), Some("json"), "envelope must be excluded: {c}");
		}
		let p = build_preview("tool_result", "{\"success\": true}");
		assert!(p.starts_with("[tool_result:"), "envelope keeps its hinted arm: {p}");
	}

	// #4: shell-script comments must not be mis-tagged as markdown.
	#[test]
	fn test_shell_comments_are_not_misdetected_as_markdown() {
		let _g = cap_guard();
		let c = "#!/bin/sh\n# build the thing\n# then run it\necho done";
		assert_eq!(detect_semantic_type(c), Some("code"), "shebang + code wins: {c}");
		let c2 = "# just a comment\n# another comment\n# third comment";
		assert_ne!(detect_semantic_type(c2), Some("markdown"), "comment-only lines are not a doc");
	}
}
