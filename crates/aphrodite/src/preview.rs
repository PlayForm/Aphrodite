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
/// Returns `Some(type)` for a recognized shape (`git`, `ls`, `test`, `grep`,
/// `gitlog`), or `None` to leave the classifier's own verdict in place. Detection
/// is deliberately conservative (line-prefix / marker patterns, majority votes)
/// so a random paragraph is never mis-tagged. Char-boundary safe and panic-free
/// on empty/NUL/multibyte input.
pub fn detect_semantic_type(content: &str) -> Option<&'static str> {
	let lines: Vec<&str> = content.lines().collect();
	if lines.is_empty() {
		return None;
	}
	let non_empty: Vec<&str> = lines.iter().map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
	if non_empty.is_empty() {
		return None;
	}

	// ── test output: cargo/pytest/jest/go ──
	// A `test result:` / `N passed` / `=== RUN` / pytest summary line is a
	// strong, unambiguous signal even amid other noise.
	if content.contains("test result:")
		|| content.contains("=== RUN ")
		|| content.contains("--- FAIL:")
		|| content.contains("--- PASS:")
		|| REEST_PYTEST.is_match(content)
		|| REEST_JEST.is_match(content)
	{
		return Some("test");
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

static REEST_PYTEST: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"\d+ passed|\d+ failed").unwrap());
static REEST_JEST: std::sync::LazyLock<regex::Regex> =
	std::sync::LazyLock::new(|| regex::Regex::new(r"Tests:\s+\d+").unwrap());

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
	// grep, git log). Both the proxy path and the Hermes hook/FFI path funnel
	// through this one function, so the enriched preview is emitted identically
	// on both paths. An explicit non-generic `type_str` is always honored as-is.
	let effective: &str = match type_str {
		"text" | "terminal" | "log" | "" | "plain" => detect_semantic_type(content).unwrap_or(type_str),
		other => other,
	};
	match effective {
		"build" | "build_output" | "build_error" => {
			let e = content.matches("error").count();
			let w = content.matches("warning").count();
			// Enrich: surface the first error MESSAGE (e.g. `E0432: unresolved
			// import ...`), not just tallies - the exact text the agent needs to
			// decide whether to retrieve the full log.
			let first_err = content
				.lines()
				.map(|l| l.trim())
				.find(|l| l.starts_with("error[") || l.starts_with("error:") || l.contains(": error["))
				.map(|l| {
					// Prefer the `error[EXXXX]: msg` / `error: msg` remainder.
					let start = l.find("error").unwrap_or(0);
					l[start..].chars().take(60).collect::<String>()
				});
			match first_err {
				Some(msg) if !msg.is_empty() => {
					format!("[build:{}E {}W {}L | {}]", e, w, lines, msg)
				},
				_ => format!("[build:{}E {}W {}L]", e, w, lines),
			}
		},
		"diff" => {
			let f = content.matches("diff --git").count();
			let a = content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
			let d = content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
			// Enrich: name the first couple of changed files so the agent sees
			// WHAT changed, not just how many lines.
			let files: Vec<String> = content
				.lines()
				.filter_map(|l| l.strip_prefix("diff --git "))
				.filter_map(|rest| rest.split_whitespace().next())
				.map(|p| p.strip_prefix("a/").unwrap_or(p).to_string())
				.take(3)
				.collect();
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
		"json_array" | "json" | "json_list" => build_json_preview(content, lines),
		// `hooks::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace, but
		// this function had no matching arm for it, so the preview silently
		// fell through to the generic `_` branch (F10) - a bare line/byte
		// count with no exit-code or last-output-line context, the exact
		// signal a terminal preview exists to surface.
		"terminal" => {
			let exit_line = content
				.lines()
				.rev()
				.find(|l| l.contains("exit code:") || l.contains("Error:"))
				.map(|l| l.trim());
			let last_line = content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim());
			let summary = exit_line.or(last_line).unwrap_or("").chars().take(60).collect::<String>();
			format!("[terminal:{}L {}]", lines, summary)
		},
		// Plain-text / unrecognized fallback: even when we can't classify the
		// shape, do better than a bare L/B count - show the first non-empty
		// line (trimmed, <=60 chars) as a content hint so the agent has SOME
		// signal about what the blob is.
		_ => {
			let hint = content
				.lines()
				.map(|l| l.trim())
				.find(|l| !l.is_empty())
				.map(|l| l.chars().take(60).collect::<String>())
				.filter(|s| !s.is_empty());
			match hint {
				Some(h) => format!("[{}:{}L {}B | {}]", type_str, lines, bytes, h),
				None => format!("[{}:{}L {}B]", type_str, lines, bytes),
			}
		},
	}
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
		let content = "before\0after\0\0end";
		let _ = detect_type(content); // must not panic
	}

	#[test]
	fn test_build_preview_never_panics_on_interior_nul_across_type_branches() {
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
		let c = "R  old/path.rs -> new/path.rs\nR  a.txt -> b.txt";
		let p = build_preview("git", c);
		assert!(p.starts_with("[git:2R | new/path.rs b.txt"), "got {p}");
	}

	const CARGO_TEST: &str = "running 221 tests\ntest foo::bar ... ok\ntest result: ok. 220 passed; 0 failed; 1 \
	                         ignored; 0 measured; 0 filtered out; finished in 0.31s";

	#[test]
	fn test_detect_and_preview_cargo_test() {
		assert_eq!(detect_semantic_type(CARGO_TEST), Some("test"));
		let p = build_preview("text", CARGO_TEST);
		assert_eq!(p, "[test:220 pass 0 fail 1 ignored | 0.31s]");
	}

	#[test]
	fn test_preview_test_names_first_failure() {
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
		assert_eq!(detect_semantic_type(RIPGREP), Some("grep"));
		let p = build_preview("text", RIPGREP);
		assert_eq!(p, "[grep:4 hits in 3 files | src/preview.rs:12 …]");
	}

	const GIT_LOG: &str = "commit abc1234def5678\nAuthor: Nikola <n@x.io>\nDate:   Mon Jul 14\n\n    fix(preview): \
	                      stop doubling\n\ncommit def5678abc1234\nAuthor: Nikola <n@x.io>\nDate:   Sun Jul 13\n\n    \
	                      feat: add detector";

	#[test]
	fn test_detect_and_preview_git_log() {
		assert_eq!(detect_semantic_type(GIT_LOG), Some("gitlog"));
		let p = build_preview("text", GIT_LOG);
		assert_eq!(
			p,
			"[gitlog:2 commits | abc1234 fix(preview): stop doubling → def5678 feat: add detector]"
		);
	}

	#[test]
	fn test_preview_build_surfaces_first_error() {
		let c = "   Compiling aphrodite v1.3.3\nerror[E0432]: unresolved import `crate::foo`\n  --> \
		         src/x.rs:1:5\nwarning: unused variable `y`";
		let p = build_preview("build", c);
		assert!(p.starts_with("[build:"), "got {p}");
		assert!(p.contains("E0432]: unresolved import"), "must surface first error text: {p}");
	}

	#[test]
	fn test_preview_diff_names_files() {
		let c = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,2 +1,3 @@\n+new\ndiff --git a/Cargo.toml \
		         b/Cargo.toml\n@@ -1 +1 @@\n-x\n+y";
		let p = build_preview("diff", c);
		assert!(p.contains("src/main.rs"), "got {p}");
		assert!(p.contains("Cargo.toml"), "got {p}");
	}

	#[test]
	fn test_fallback_shows_first_line_hint() {
		let c = "some unrecognizable prose here\nline two\nline three";
		let p = build_preview("text", c);
		// No semantic shape detected -> generic fallback WITH a first-line hint.
		assert_eq!(p, "[text:3L 50B | some unrecognizable prose here]");
	}

	#[test]
	fn test_semantic_detector_leaves_prose_alone() {
		let prose = "The quick brown fox jumps over the lazy dog.\nAnother sentence of ordinary prose.";
		assert_eq!(detect_semantic_type(prose), None);
	}

	// ── JSON preview (intelligent parsing, not crude `{"` count) ──

	#[test]
	fn test_json_preview_array_shows_keys() {
		let c = "[{\"status\":\"ok\",\"count\":42},{\"status\":\"err\",\"count\":0}]";
		let p = build_preview("json_array", c);
		assert_eq!(p, "[json:2items 1L | keys: status, count]");
	}

	#[test]
	fn test_json_preview_object_shows_keys() {
		let c = "{\"status\":\"in_progress\",\"conclusion\":null,\"jobs\":[{\"name\":\"Test\"}]}";
		let p = build_preview("json", c);
		assert!(p.starts_with("[json:3keys"));
		assert!(p.contains("status"));
		assert!(p.contains("conclusion"));
		assert!(p.contains("jobs"));
	}

	#[test]
	fn test_json_preview_fallback_on_unparseable() {
		let c = "not valid json {at all";
		let p = build_preview("json_array", c);
		assert!(p.starts_with("[json:~"));
	}

	// ── Search preview (proper regex, not bare `:` count) ──

	#[test]
	fn test_search_preview_counts_hits_and_files() {
		let c = "src/main.rs:12:    let x = 1;\nsrc/main.rs:42:    println!();\nsrc/lib.rs:7:    pub fn foo()";
		let p = build_preview("search", c);
		assert!(p.starts_with("[search:3 hits in 2 files | "), "got {p}");
	}

	#[test]
	fn test_search_preview_ignores_non_search_lines() {
		let c = "src/main.rs:12:    let x = 1;\njust some prose here\nsrc/lib.rs:7:    pub fn foo()";
		let p = build_preview("search", c);
		assert!(p.contains("2 hits"), "got {p}");
	}

	// ── HTML preview (title, headings, links, images) ──

	#[test]
	fn test_html_preview_extracts_title_and_counts() {
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
}
