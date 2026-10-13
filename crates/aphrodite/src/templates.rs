//! TOML-driven preview template rendering (pre-Rust port).
//!
//! Port of `plugins/aphrodite/_core/template.py::_render_template`: when
//! `[templates.preview.<family>.<ctype>]` format strings are configured they
//! render instead of the hardcoded per-arm preview builders. Resolution per
//! content type: `family[ctype]` → `[templates.reverse][ctype]` mapped key →
//! `family[_default]` → `None` (the caller falls back to the hardcoded
//! builder). `{variable}` substitution mirrors the pre-Rust variable set;
//! unknown variables degrade to empty string - never a panic.
//!
//! Registration mirrors the `preview/state.rs` `set_preview_max_chars`
//! pattern: a process-global registry set by `config_loader::apply_templates`
//! (dylib path), `main.rs` (engine binary), and the proxy `/reload` handler
//! (hot-reload).

use std::{
	collections::HashMap,
	sync::{OnceLock, RwLock},
};

use crate::state::TemplatesConfig;

/// Process-global template registry: the parsed `[templates]` section plus
/// the `[previews] model_family` selector (default `"code_first"` when
/// unset). An unknown family degrades to `None` - the caller keeps the
/// hardcoded builders, never a panic.
struct TemplatesState {
	templates:Option<TemplatesConfig>,
	family:Option<String>,
}

static TEMPLATES:OnceLock<RwLock<TemplatesState>> = OnceLock::new();

fn state() -> &'static RwLock<TemplatesState> {
	TEMPLATES.get_or_init(|| RwLock::new(TemplatesState { templates:None, family:None }))
}

/// Register the `[templates]` config (None = hardcoded builders remain the
/// default). Returns the previous value so callers (tests, hot-reload) can
/// restore it - same pattern as `set_preview_max_chars`.
pub fn set_templates(templates:Option<TemplatesConfig>) -> Option<TemplatesConfig> {
	let mut st = state().write().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::mem::replace(&mut st.templates, templates)
}

/// Currently registered `[templates]` config (None = hardcoded builders).
pub fn get_templates() -> Option<TemplatesConfig> {
	state()
		.read()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
		.templates
		.clone()
}

/// Register the `[previews] model_family` selector; None = default
/// (`"code_first"`). Returns the previous value (restore-friendly).
pub fn set_preview_family(family:Option<String>) -> Option<String> {
	let mut st = state().write().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::mem::replace(&mut st.family, family)
}

/// Currently registered `[previews] model_family` selector.
pub fn get_preview_family() -> Option<String> {
	state().read().unwrap_or_else(std::sync::PoisonError::into_inner).family.clone()
}

/// Render the configured `[templates.preview]` template for `type_str` +
/// `content`, or `None` when nothing is configured for this type (the caller
/// then runs the hardcoded per-arm builder unchanged).
///
/// Resolution (pre-Rust `_render_template`): the family map is selected by
/// `[previews] model_family` (default `"code_first"` when unset), then
/// `family[ctype]` → `reverse[ctype]` → `family[_default]` → `None`. An
/// empty resolved template also degrades to `None` (the pre-Rust falsy
/// fallback). Substitution is defensive: unknown `{variable}`s render as
/// empty strings and a malformed template never panics.
pub fn render_preview(type_str:&str, content:&str) -> Option<String> {
	let st = state().read().unwrap_or_else(std::sync::PoisonError::into_inner);
	let templates = st.templates.as_ref()?;
	let family = st.family.clone().unwrap_or_else(|| "code_first".to_string());
	let family_templates = templates.preview.get(&family)?;
	let tmpl = family_templates
		.get(type_str)
		.or_else(|| templates.reverse.get(type_str).and_then(|mapped| family_templates.get(mapped)))
		.or_else(|| family_templates.get("_default"))?;
	if tmpl.trim().is_empty() {
		return None;
	}
	let vars = build_vars(type_str, content);
	Some(substitute(tmpl, &vars))
}

/// Build the template variable set from content, mirroring the pre-Rust
/// `_make_ccr_preview` vars dict (plus its `fx`/`cmx`/`sig1` computed vars).
/// Every extraction is defensive: a value that cannot be derived from the
/// content degrades to its pre-Rust default (`"?"`/`""`/`"0"`), never panics.
fn build_vars(type_str:&str, content:&str) -> HashMap<String, String> {
	let lines:Vec<&str> = content.lines().collect();
	let ln = lines.len();
	let lower:Vec<String> = lines.iter().map(|l| l.to_ascii_lowercase()).collect();

	let first = content
		.chars()
		.take(110)
		.collect::<String>()
		.replace(['\n', '\r'], " ")
		.trim()
		.to_string();
	let err_count = lower.iter().filter(|l| l.contains("error")).count();
	let warn_count = lower.iter().filter(|l| l.contains("warning")).count();
	let msg = lines
		.iter()
		.zip(lower.iter())
		.find(|(_, l)| l.contains("error") || l.contains("failed"))
		.map(|(l, _)| l.trim().chars().take(110).collect::<String>())
		.unwrap_or_default();
	let exit = extract_exit(&lines);
	let cmd = extract_cmd(&lines);
	let commit = extract_commit(&lines);
	let (plus, minus) = diff_counts(&lines);

	let mut vars:HashMap<String, String> = HashMap::new();
	vars.insert("type".into(), type_str.to_string());
	vars.insert("ln".into(), ln.to_string());
	vars.insert("size".into(), content.len().to_string());
	vars.insert("first".into(), first);
	vars.insert("err".into(), err_count.to_string());
	vars.insert("warn".into(), warn_count.to_string());
	vars.insert("msg".into(), msg);
	vars.insert("exit".into(), exit);
	vars.insert("cmd".into(), cmd.clone());
	vars.insert("files".into(), "?".into());
	vars.insert("fn".into(), String::new());
	vars.insert("plus".into(), plus);
	vars.insert("minus".into(), minus);
	vars.insert("keys".into(), String::new());
	vars.insert("items".into(), "?".into());
	vars.insert("commit".into(), commit.clone());
	vars.insert("hash".into(), commit);
	vars.insert("subject".into(), String::new());
	vars.insert("pid".into(), "?".into());
	vars.insert("uptime".into(), String::new());
	// Code-structure defaults (pre-Rust): fns "?", structs/impls/classes "0".
	vars.insert("fns".into(), "?".into());
	vars.insert("structs".into(), "0".into());
	vars.insert("impls".into(), "0".into());
	vars.insert("classes".into(), "0".into());
	vars.insert("sigs".into(), String::new());
	// Pre-Rust extras + computed vars.
	vars.insert("code".into(), "?".into());
	vars.insert("loc".into(), String::new());
	vars.insert("errors".into(), err_count.to_string());
	vars.insert("total".into(), ln.to_string());

	// Structure-map enrichment for code content (pre-Rust: fns/structs/
	// impls/classes/sigs filled from `_extract_code_structure`).
	if matches!(
		type_str,
		"code" | "source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh"
	) {
		let lang = match type_str {
			"code_rust" => "rust",
			"code_python" => "python",
			"code_go" => "go",
			"code_js" => "js",
			"code_ts" => "js",
			"code_sh" => "sh",
			_ => "",
		};
		let st = crate::struct_extract::extract_code_structure(content, lang);
		let sigs = st
			.get("fns")
			.map(|v| v.iter().take(2).cloned().collect::<Vec<_>>().join("; "))
			.unwrap_or_default();
		vars.insert(
			"fns".into(),
			st.get("fns").map(|v| v.len().to_string()).unwrap_or_else(|| "?".into()),
		);
		vars.insert(
			"structs".into(),
			st.get("structs").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
		);
		vars.insert(
			"impls".into(),
			st.get("impls").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
		);
		vars.insert(
			"classes".into(),
			st.get("classes").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
		);
		vars.insert("sigs".into(), sigs.clone());
		// Computed `sig1` = first signature (pre-Rust).
		vars.insert(
			"sig1".into(),
			sigs.split(';').next().map(|s| s.trim().to_string()).unwrap_or_default(),
		);
	}

	// Computed vars (pre-Rust): `fx` = " {fn[:40]}" when fn set; `cmx` =
	// command stripped of leading `$`/`>` then truncated to 40 chars, "?"
	// when no command. `fn` is never derivable from content alone, so `fx`
	// stays empty; `cmx` derives from the extracted command.
	let cmx = if cmd.is_empty() { "?".to_string() } else { cmd.chars().take(40).collect() };
	vars.insert("fx".into(), String::new());
	vars.insert("cmx".into(), cmx);
	if !vars.contains_key("sig1") {
		vars.insert("sig1".into(), String::new());
	}
	vars
}

/// First `EXIT:<digits>` marker in the content (the house shell-trace
/// convention `echo "EXIT:$?"`); `"?"` when absent.
fn extract_exit(lines:&[&str]) -> String {
	for l in lines {
		if let Some((_, rest)) = l.split_once("EXIT:") {
			let code:String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
			if !code.is_empty() {
				return code;
			}
		}
	}
	"?".to_string()
}

/// First command-looking line (trimmed start `$` or `>`), stripped of its
/// leading `$`/`>` and whitespace, truncated to 200 chars; `""` when absent.
fn extract_cmd(lines:&[&str]) -> String {
	for l in lines {
		let t = l.trim_start();
		if let Some(rest) = t.strip_prefix('$').or_else(|| t.strip_prefix('>')) {
			let cmd = rest.trim();
			if !cmd.is_empty() {
				return cmd.chars().take(200).collect();
			}
		}
	}
	String::new()
}

/// First 7-40 hex run at a line start (git log short/full hash), or the
/// hash of a `commit <hex>` line; pre-Rust default `"???????"`.
fn extract_commit(lines:&[&str]) -> String {
	for l in lines {
		let t = l.trim_start();
		let candidate = t.strip_prefix("commit ").map(str::trim).unwrap_or(t);
		let hex:String = candidate.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
		if hex.len() >= 7 && hex.len() <= 40 {
			return hex.chars().take(7).collect();
		}
	}
	"???????".to_string()
}

/// `+`/`-` line tallies for diff content (excluding `+++`/`---` hunk
/// headers); `("?", "?")` when the content has no diff lines at all.
fn diff_counts(lines:&[&str]) -> (String, String) {
	let plus = lines.iter().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
	let minus = lines.iter().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
	if plus == 0 && minus == 0 {
		("?".into(), "?".into())
	} else {
		(plus.to_string(), minus.to_string())
	}
}

/// `{variable}` substitution. Every `{name}` is replaced by the registered
/// value; an unknown name degrades to an empty string; a lone `{` with no
/// closing brace is passed through verbatim. Never panics on any input.
fn substitute(tmpl:&str, vars:&HashMap<String, String>) -> String {
	let mut out = String::with_capacity(tmpl.len());
	let mut rest = tmpl;
	while let Some(open) = rest.find('{') {
		out.push_str(&rest[..open]);
		let after = &rest[open + 1..];
		match after.find('}') {
			Some(close) => {
				let name = &after[..close];
				if name.is_empty() {
					out.push_str("{}");
				} else {
					out.push_str(vars.get(name).map(String::as_str).unwrap_or(""));
				}
				rest = &after[close + 1..];
			},
			None => {
				out.push('{');
				out.push_str(after);
				rest = "";
			},
		}
	}
	out.push_str(rest);
	out
}

/// Serializes tests across modules that mutate the process-global template
/// registry (`cargo test` runs module test-bodies concurrently; the registry
/// is process-wide, so `config_loader`'s `apply_templates` test and this
/// module's tests must not interleave).
#[cfg(test)]
pub(crate) fn templates_test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:OnceLock<std::sync::Mutex<()>> = OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fam(entries:&[(&str, &str)]) -> HashMap<String, String> {
		entries.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
	}

	/// Family resolution + reverse map + `_default` fallback + unknown-family
	/// fall-through + unknown-variable degradation, in one battery.
	#[test]
	fn test_render_preview_family_reverse_default_unknown_var() {
		let _g = templates_test_guard();
		let mut preview = HashMap::new();
		preview.insert(
			"code_first".to_string(),
			fam(&[
				("diff", "{type}:+{plus}/-{minus} {ln}L [{zzz}]"),
				("build", "[build {err}err {warn}warn {msg}]"),
				("grep", "[grep {items} hits {nope}]"),
				("_default", "[{type} {ln}L {size}B]"),
			]),
		);
		let mut reverse = HashMap::new();
		reverse.insert("build_output".to_string(), "build".to_string());
		set_templates(Some(TemplatesConfig {
			preview,
			marker:HashMap::new(),
			prompts:HashMap::new(),
			reverse,
		}));
		set_preview_family(Some("code_first".to_string()));

		// family[ctype] direct hit, with an unknown {zzz} degrading to "".
		let r = render_preview("diff", "diff --git a/x b/x\n+a\n+b\n-c\n").expect("diff template");
		assert_eq!(r, "diff:+2/-1 4L []");

		// reverse[ctype] mapped hit: build_output -> build.
		let r = render_preview("build_output", "error: boom\nwarning: meh\n").expect("reverse map");
		assert_eq!(r, "[build 1err 1warn error: boom]");

		// family[_default] fallback: ls has no entry, _default renders.
		let r = render_preview("ls", "a.txt\nb.txt\n").expect("_default fallback");
		assert_eq!(r, "[ls 2L 12B]");

		// Unknown variables degrade to empty string.
		let r = render_preview("grep", "hit\n").expect("grep template");
		assert_eq!(r, "[grep ? hits ]");

		// Unknown family -> None (caller falls back to the hardcoded builder).
		set_preview_family(Some("nonexistent".to_string()));
		assert!(render_preview("diff", "x").is_none());

		// Family unset -> default "code_first" (the registered family). No
		// +/- lines => pre-Rust defaults "?"/"?".
		set_preview_family(None);
		assert_eq!(render_preview("diff", "a\nb\nc\n").as_deref(), Some("diff:+?/-? 3L []"));

		// No templates registered -> None.
		set_templates(None);
		assert!(render_preview("diff", "x").is_none());

		// Restore globals for later tests.
		set_preview_family(None);
	}

	/// `set_templates`/`get_templates` round-trip + `get_preview_family`.
	#[test]
	fn test_templates_registry_roundtrip() {
		let _g = templates_test_guard();
		set_templates(None);
		assert!(get_templates().is_none());
		let mut preview = HashMap::new();
		preview.insert("code_first".to_string(), fam(&[("_default", "[{type} {ln}L]")]));
		let cfg = TemplatesConfig { preview, marker:HashMap::new(), prompts:HashMap::new(), reverse:HashMap::new() };
		let prev = set_templates(Some(cfg));
		assert!(prev.is_none());
		assert!(get_templates().is_some());
		assert_eq!(set_preview_family(Some("balance".to_string())), None);
		assert_eq!(get_preview_family().as_deref(), Some("balance"));
		set_templates(None);
		set_preview_family(None);
	}

	/// End-to-end (task deliverable 2): `build_preview` consults the
	/// registered templates FIRST, applies the `preview_max_chars` cap to the
	/// rendered result, and falls back to the hardcoded per-arm dispatch when
	/// nothing is registered.
	#[test]
	fn test_build_preview_consults_registered_templates() {
		let _g = templates_test_guard();
		let _cap = crate::preview::preview_cap_test_guard();
		let mut fam_map = HashMap::new();
		fam_map.insert("diff".to_string(), "[tpl {type} {ln}L]".to_string());
		let mut preview = HashMap::new();
		preview.insert("code_first".to_string(), fam_map);
		set_templates(Some(TemplatesConfig {
			preview,
			marker:HashMap::new(),
			prompts:HashMap::new(),
			reverse:HashMap::new(),
		}));
		set_preview_family(Some("code_first".to_string()));

		// Template renders first; the cap (8 chars) truncates it on a char
		// boundary, preserving the closing `]`.
		crate::preview::set_preview_max_chars(Some(8));
		let out = crate::preview::build_preview("diff", "a\nb\n");
		assert_eq!(out, "[tpl d…]");

		// Unregistered -> hardcoded diff arm, unchanged.
		set_templates(None);
		let out = crate::preview::build_preview("diff", "a\nb\n");
		assert!(out.starts_with("[diff"), "hardcoded fallback expected, got {out:?}");

		// Restore globals for later tests.
		crate::preview::set_preview_max_chars(None);
		set_preview_family(None);
	}
}
