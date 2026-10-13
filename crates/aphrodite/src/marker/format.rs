use std::{
	collections::HashMap,
	sync::{
		RwLock,
		atomic::{AtomicBool, Ordering},
	},
};

/// Build a CCR output block.
///
/// - `hash_val`: the content hash
/// - `ccr_type`: content type string (e.g. "code_rust", "build")
/// - `size`: original content size in bytes
/// - `preview`: the formatted preview string
/// - `headroom_budget`: optional token budget for truncation
/// - `meta`: optional metadata key-value pairs
/// - `center`: optional center annotation
///
/// The rendered block consults the process-global `[templates.marker]`
/// registration ([`set_marker_templates`]): when a `format` string is
/// configured it is rendered with `{preview} {type} {meta} {center}
/// {center_seg} {hash} {size} {hint}` substitution; otherwise the fixed
/// `<<<CCR:{hash}|{type}|{size}>>>` grammar is the fallback.
pub fn ccr_marker(
	hash_val:&str,
	ccr_type:&str,
	size:usize,
	preview:&str,
	headroom_budget:Option<u32>,
	meta:Option<&HashMap<String, String>>,
	center:Option<&str>,
) -> String {
	// Sanitize preview: newlines → spaces (| is safe - the marker is on its own line).
	let mut safe = preview.replace(['\n', '\r'], " ").trim().to_string();
	// Strip control chars
	safe = safe.chars().filter(|c| *c >= ' ').collect();

	// Headroom budget truncation
	if let Some(budget) = headroom_budget {
		safe = if budget < 25 {
			safe.chars().take(30).collect()
		} else if budget < 50 {
			safe.chars().take(60).collect()
		} else if budget < 75 {
			safe.chars().take(100).collect()
		} else {
			safe
		};
	}

	// Metadata string
	let meta_str = if let Some(m) = meta {
		let parts:Vec<String> = m
			.iter()
			.filter_map(|(k, v)| {
				let sv = v.replace('|', "/").replace('\n', " ").trim().to_string();
				if sv.is_empty() { None } else { Some(format!("{}={}", k, sv)) }
			})
			.collect();
		let mut s = parts.join(";");
		if s.len() > 300 {
			s = format!("{}...", crate::struct_extract::floor_boundary(&s, 297));
		}
		s
	} else {
		String::new()
	};

	render_marker_block(&safe, ccr_type, &meta_str, center, hash_val, size)
}

// ── TOML-driven marker block templates (pre-Rust port) ──────────────────
//
// Port of `plugins/aphrodite/_core/template.py` `_render_marker_tmpl`:
// `[templates.marker] format` + `hint` strings, with the `{hint}` var only
// populated when the `[prompts] ccr_marker_hint` knob is enabled. The
// registration mirrors `preview::set_preview_max_chars`: the marker builder
// runs on paths with no `AphroditeState` (hooks, dylib, poll worker), so
// the configured strings live process-globally.

/// Registered `[templates.marker] format` string; `None` = the canonical
/// fallback grammar.
static MARKER_FORMAT:RwLock<Option<String>> = RwLock::new(None);
/// Registered `[templates.marker] hint` text; emitted only when the
/// `ccr_marker_hint` knob is enabled (default off).
static MARKER_HINT:RwLock<String> = RwLock::new(String::new());
/// `[prompts] ccr_marker_hint` knob - hint text is emitted only when true.
static MARKER_HINT_ENABLED:AtomicBool = AtomicBool::new(false);

/// Register the TOML-driven marker block templates.
///
/// Mirrors the `preview::set_preview_max_chars` registration pattern. Set
/// by: the engine binary right after `MultiConfig::load` (main.rs), the
/// proxy `/reload` handler (proxy.rs), and
/// `config_loader::Config::apply_compression` at dylib init. `None` format
/// (or an absent `[templates]` section) resets to the fixed
/// `<<<CCR:{hash}|{type}|{size}>>>` fallback so a hot-reload that removes
/// the templates never leaves a stale format in force.
pub fn set_marker_templates(format:Option<&str>, hint:Option<&str>, hint_enabled:bool) {
	*MARKER_FORMAT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = format.map(|s| s.to_string());
	*MARKER_HINT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = hint.unwrap_or("").to_string();
	MARKER_HINT_ENABLED.store(hint_enabled, Ordering::Relaxed);
}

/// Currently registered marker format (test/visibility helper).
pub(crate) fn registered_marker_format() -> Option<String> {
	MARKER_FORMAT.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Whether the `ccr_marker_hint` knob is enabled (test/visibility helper).
pub(crate) fn marker_hint_enabled() -> bool { MARKER_HINT_ENABLED.load(Ordering::Relaxed) }

/// The hint text to emit right now: empty unless the `ccr_marker_hint` knob
/// is enabled AND a hint string is registered (port of Python's
/// `hint = hint_str if CCR_MARKER_HINT and hint_str else ""`).
fn current_hint() -> String {
	if !marker_hint_enabled() {
		return String::new();
	}
	MARKER_HINT.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
}

/// Dispatch marker rendering: configured `[templates.marker] format` wins,
/// the canonical three-part grammar is the fallback.
fn render_marker_block(preview:&str, ccr_type:&str, meta:&str, center:Option<&str>, hash:&str, size:usize) -> String {
	let hint = current_hint();
	match registered_marker_format() {
		Some(fmt) => {
			let size_s = size.to_string();
			let center_seg = center.map(|c| format!(";center={c}")).unwrap_or_default();
			let vars = [
				("preview", preview),
				("type", ccr_type),
				("meta", meta),
				("center", center.unwrap_or("")),
				("center_seg", center_seg.as_str()),
				("hash", hash),
				("size", size_s.as_str()),
				("hint", hint.as_str()),
			];
			substitute_vars(&fmt, &vars)
		},
		None => render_marker_fallback(preview, ccr_type, meta, center, hash, size, &hint),
	}
}

/// Render the canonical marker grammar (fallback when `[templates.marker]
/// format` is unconfigured).
///
/// Doubling-bug fix (report 09 §5): `build_preview` already returns a
/// self-describing, fully bracketed preview like `[text:53L 1913B]` or
/// `[git:5M 2A | src/x.rs]`. Re-wrapping that in `[{center_str}:{preview}]`
/// produced the visible `[text:[text:53L 1913B]]` doubling. When the preview
/// is already a `[label:...]`-shaped string, emit it verbatim on the preview
/// line; only wrap bare previews (e.g. cache-mode raw excerpts) in the
/// `[{center_str}:...]` frame. A preview is thus produced exactly once.
///
/// The `hint` text is appended to the preview line only when the
/// `ccr_marker_hint` knob is enabled (opt-in; empty = byte-identical to the
/// pre-knob output).
fn render_marker_fallback(
	preview:&str,
	ccr_type:&str,
	meta:&str,
	center:Option<&str>,
	hash:&str,
	size:usize,
	hint:&str,
) -> String {
	let center_str = center.unwrap_or(ccr_type);
	let meta_part = if meta.is_empty() { String::new() } else { format!("\n[meta:{}]", meta) };

	let preview_line = if is_self_bracketed_preview(preview) {
		preview.to_string()
	} else {
		format!("[{}:{}]", center_str, preview)
	};

	format!(
		"<<<CCR:{}|{}|{}>>>\n{}{}{}",
		hash, ccr_type, size, preview_line, hint, meta_part
	)
}

/// True when `preview` is already a self-describing `[label:...]` preview (as
/// produced by `build_preview`), so `render_marker` must not wrap it again.
/// Requires a leading `[`, a matching trailing `]`, and a `[word:` label head
/// (`\\[\\w+:`) so a bare excerpt that merely happens to start with `[` isn't
/// mistaken for a preview.
fn is_self_bracketed_preview(preview:&str) -> bool {
	let p = preview.trim();
	if !p.starts_with('[') || !p.ends_with(']') {
		return false;
	}
	let inner = &p[1..];
	match inner.find(':') {
		Some(colon) => {
			let label = &inner[..colon];
			!label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
		},
		None => false,
	}
}

/// Substitute `{name}` placeholders in a template string - the Rust port of
/// the pre-Rust Python `str.format(**safe)` semantics used by
/// `_core/template.py` (`_render_marker_tmpl`, `_render_prompt_tmpl`).
///
/// - `{{` / `}}` escape to literal braces (Python str.format).
/// - Known variable names are replaced by their string values.
/// - Unknown placeholders, malformed groups (e.g. `{hash!r}`), and stray
///   braces are emitted verbatim: Python raised KeyError/ValueError (a
///   crash in `_render_marker_tmpl`, a raw-template fallback in
///   `_render_prompt_tmpl`); this crate degrades without panicking and
///   keeps the template readable.
pub(crate) fn substitute_vars(tmpl:&str, vars:&[(&str, &str)]) -> String {
	let chars:Vec<char> = tmpl.chars().collect();
	let mut out = String::with_capacity(tmpl.len() + 16);
	let mut i = 0;
	while i < chars.len() {
		let c = chars[i];
		if c == '{' {
			if i + 1 < chars.len() && chars[i + 1] == '{' {
				out.push('{');
				i += 2;
				continue;
			}
			// Try to parse `{name}` with a clean [A-Za-z0-9_] name.
			let mut j = i + 1;
			let mut name = String::new();
			while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
				name.push(chars[j]);
				j += 1;
			}
			if !name.is_empty() && j < chars.len() && chars[j] == '}' {
				if let Some((_, v)) = vars.iter().find(|(k, _)| *k == name.as_str()) {
					out.push_str(v);
					i = j + 1;
					continue;
				}
			}
			// Unknown var or malformed group: emit the opening brace
			// verbatim and keep scanning - the rest of the group text is
			// then emitted literally.
			out.push('{');
			i += 1;
			continue;
		}
		if c == '}' {
			if i + 1 < chars.len() && chars[i + 1] == '}' {
				out.push('}');
				i += 2;
				continue;
			}
			out.push('}');
			i += 1;
			continue;
		}
		out.push(c);
		i += 1;
	}
	out
}

/// Serializes tests across modules that mutate the process-global marker
/// templates (`cargo test` runs module test-bodies concurrently; the
/// registration is process-wide, so config_loader's `apply_compression`
/// tests and this module's template tests must not interleave).
#[cfg(test)]
pub(crate) fn marker_templates_test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
