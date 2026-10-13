//! Full hook implementations - expanded from plugins/aphrodite/_hooks/
//!
//! transform_tool_result: content-aware compression with essential tool skip,
//!   file reference tracking, threshold gating, preview generation.
//! transform_terminal_output: terminal-specific compression with exit code
//!   detection, threshold gating, streaming support.
//! auto-expand: when `state.auto_expand` is on and content fits the limit,
//!   both hooks store + record the compression exactly as today but return
//!   the RESOLVED content inline instead of the CCR marker, so no
//!   `<<<CCR:...>>>` marker reaches the model (dylib-side consumer of
//!   `[compression] auto_expand`; the bridge keys on the `auto_expanded`
//!   flag / `expanded` string added to the payload).

use headroom_core::transforms;

use crate::{
	marker::ccr_marker,
	state::{AphroditeState, MarkerEntry, ToolEvent},
};

/// Metadata Hermes already sends alongside a tool result (report 05, P2) - the
/// signals the bridge historically dropped on the floor (`status`,
/// `error_type`, `error_message`, `args`, `duration_ms`). Threaded through
/// [`transform_tool_result_with_meta`] into the `tool_events` telemetry ring.
#[derive(Debug, Default, Clone, Copy)]
pub struct ToolCallMeta<'a> {
	pub args_json:Option<&'a serde_json::Value>,
	pub status:Option<&'a str>,
	pub error_type:Option<&'a str>,
	pub error_message:Option<&'a str>,
	pub duration_ms:Option<u64>,
}

/// Compute a CCR hash for content using BLAKE3 (40 hex chars).
pub fn compute_hash(content:&str) -> String { headroom_core::ccr::compute_key(content.as_bytes()) }

/// Auto-expand gate (dylib-side consumer of `[compression] auto_expand`):
/// when enabled AND the content fits `auto_expand_limit` bytes, the
/// transform hooks return the resolved content inline instead of the CCR
/// marker, so no `<<<CCR:...>>>` marker reaches the model. Over the cap (or
/// disabled) the caller keeps today's marker behavior unchanged.
/// `limit == 0` is effectively off (no non-empty content fits), matching the
/// pre-Rust `AUTO_EXPAND_LIMIT = 0` = OFF semantics.
fn auto_expand_applies(state:&AphroditeState, content:&str) -> bool {
	state.auto_expand && content.len() <= state.auto_expand_limit
}

/// Resolve a freshly stored hash through the crate's recursive resolution
/// entry point (port of the pre-Rust `_resolve_recursive` auto-expand path).
/// Call only when [`auto_expand_applies`] is true. Resolution failure
/// degrades to the original content (warn + degrade house style, never
/// panic) - the payload still carries a valid marker for manual retrieval
/// either way.
fn auto_expand_content(state:&mut AphroditeState, hash:&str, content:&str) -> String {
	match crate::resolve::expand(state, hash) {
		Some(expanded) => expanded,
		None => {
			tracing::warn!(
				hash,
				"auto-expand: inline resolve failed for freshly stored hash; returning original content"
			);
			content.to_string()
		},
	}
}

// ── Live containers (pre-Rust port of _hooks/live.py) ─────────────────
//
// When `state.live_container` is on AND content clears the compress
// threshold, the transform hooks store the content as a live container -
// per-chunk entries + a metadata envelope - and emit the INSTANT marker
// with a progress annotation. The retrieve path then serves the chunks
// progressively (chunk count / bytes arrived via the container status);
// a final resolve returns the full content. The content is ALSO stored
// under the plain hash so `resolve::expand` keeps the lossless
// round-trip contract. Off (the default) = byte-identical current
// behavior.

/// Target byte size of one live-container chunk. Chunking cuts on UTF-8
/// char boundaries, so a chunk may be a few bytes under this when a
/// multi-byte character straddles the boundary.
pub(crate) const LIVE_CHUNK_SIZE:usize = 1024;
/// Inline-store key suffix of the container metadata envelope.
pub(crate) const LIVE_META_SUFFIX:&str = ":live:meta";
/// Inline-store key segment prefix of the per-chunk entries.
pub(crate) const LIVE_CHUNK_SUFFIX:&str = ":live:chunk:";

/// Inline-store key of a live container's metadata envelope.
pub(crate) fn live_meta_key(hash:&str) -> String { format!("{hash}{LIVE_META_SUFFIX}") }

/// Inline-store key of a live container's `index`-th chunk.
pub(crate) fn live_chunk_key(hash:&str, index:usize) -> String { format!("{hash}{LIVE_CHUNK_SUFFIX}{index}") }

/// Split `content` into chunks of at most `chunk_size` BYTES, cutting only
/// on UTF-8 char boundaries so reassembly is lossless - a raw byte slice
/// can split a multi-byte char, and lossy re-encoding would corrupt the
/// content and break the round-trip contract.
pub(crate) fn chunk_on_char_boundaries(content:&str, chunk_size:usize) -> Vec<&str> {
	let mut chunks = Vec::new();
	let mut start = 0usize;
	for (idx, _ch) in content.char_indices() {
		if idx >= start + chunk_size {
			chunks.push(&content[start..idx]);
			start = idx;
		}
	}
	if start < content.len() {
		chunks.push(&content[start..]);
	}
	chunks
}

/// Render the instant live-container marker: the standard CCR header with
/// the `live` type, then a single annotation line carrying the progress
/// handle - chunk count, total size, and bytes arrived so far (0 at emit
/// time; the retrieve path advances it). Sanitizes the preview the same
/// way `ccr_marker` does (newlines → spaces).
fn live_marker(hash:&str, total:usize, chunks:usize, preview:&str) -> String {
	let safe = preview.replace(['\n', '\r'], " ").trim().to_string();
	let chunk_note = if chunks == 1 { "1 chunk".to_string() } else { format!("{chunks} chunks") };
	format!(
		"<<<CCR:{}|live|{}>>>\nLive container: {} · {} · {} bytes · 0 bytes arrived. Use aphrodite_retrieve({}) to \
		 fetch when needed. Continue reasoning without waiting.",
		hash, total, safe, chunk_note, total, hash
	)
}

/// Live-container transform (pre-Rust port of `_wrap_as_live_container`):
/// store the content chunked with a metadata envelope, record a `live`
/// marker entry, and return the INSTANT marker payload with a progress
/// annotation. Call only when `state.live_container` is on and the content
/// cleared the compress threshold (and auto-expand is not taking over).
fn transform_live_container(state:&mut AphroditeState, content:&str, type_str:&str) -> serde_json::Value {
	let hash = headroom_core::ccr::compute_key(content.as_bytes());
	let total = content.len();
	let chunks = chunk_on_char_boundaries(content, LIVE_CHUNK_SIZE);
	let n = chunks.len();

	// Round-trip entry: the plain hash resolves to the exact original via
	// `resolve::expand` (and is what a final full retrieve serves).
	state.inline_store_put(hash.clone(), content.to_string());
	// Container entries: per-chunk content + the metadata envelope.
	// `bytes_arrived` starts at 0 - the marker is instant, delivery happens
	// through the retrieve path, which advances the counter.
	for (i, chunk) in chunks.iter().enumerate() {
		state.inline_store_put(live_chunk_key(&hash, i), (*chunk).to_string());
	}
	let meta = serde_json::json!({
		"live": true,
		"type": type_str,
		"size": total,
		"chunks": n,
		"chunk_size": LIVE_CHUNK_SIZE,
		"bytes_arrived": 0,
	});
	state.inline_store_put(live_meta_key(&hash), meta.to_string());

	let preview = crate::build_preview(type_str, content);
	let marker = live_marker(&hash, total, n, &preview);

	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:"live".to_string(),
		size:total,
		preview:preview.clone(),
		turn:state.turn_counter,
		center:None,
		meta:None,
	});

	serde_json::json!({
		"status": "ok",
		"compressed": true,
		"live": true,
		"hash": hash,
		"type": "live",
		"size": total,
		"chunks": n,
		"bytes_arrived": 0,
		"preview": preview,
		"marker": marker,
	})
}

/// Essential tools that must NOT be compressed - agent needs raw output.
const ESSENTIAL_TOOLS:&[&str] = &[
	"skill_view",
	"skills_list",
	"skill_manage",
	"memory",
	"session_search",
	"read_file",
	"read_terminal",
];

/// Transform tool output - full compression pipeline.
pub fn transform_tool_result(state:&mut AphroditeState, content:&str, tool_name:&str) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, None, &ToolCallMeta::default())
}

/// Same as [`transform_tool_result`], but a caller that has already found the
/// "real" payload underneath a wrapper the core classifier can't see through
/// (e.g. an agent-specific JSON envelope) may supply it via `classify` as
/// `(content_to_classify, type)`. Core stays agnostic to what a wrapper looks
/// like - it only ever hashes and stores the ORIGINAL `content`, so
/// `aphrodite_retrieve` always returns exactly what was passed in; `classify`
/// affects only the reported `type` and the generated preview.
pub fn transform_tool_result_classified(
	state:&mut AphroditeState,
	content:&str,
	tool_name:&str,
	classify:Option<(&str, &str)>,
) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, classify, &ToolCallMeta::default())
}

/// Same as [`transform_tool_result_classified`], plus the Hermes-supplied call
/// metadata (report 05, P2/T6) that gets recorded into the `tool_events`
/// telemetry ring. This is the entry point the Hermes bridge routes to so the
/// error/args/duration signals Hermes ships on every call stop being dropped.
pub fn transform_tool_result_with_meta(
	state:&mut AphroditeState,
	content:&str,
	tool_name:&str,
	classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
	transform_tool_result_inner(state, content, tool_name, classify, meta)
}

/// Record a per-call telemetry event from the supplied metadata (P2/T6).
/// `ok` is fail-open: a missing `status` means success. `wrote_path` is
/// inferred from the args of `write_file`/`patch`-style tools.
fn record_tool_event_from_meta(state:&mut AphroditeState, tool_name:&str, content:&str, meta:&ToolCallMeta) {
	let ok = meta.status.map(|s| s != "error").unwrap_or(true);
	let error_sig = if ok {
		None
	} else {
		Some(crate::flow::error_sig(meta.error_type, meta.error_message))
	};
	let wrote_path = wrote_path_from(tool_name, meta.args_json, content);
	let sig = crate::flow::normalize_args_sig(tool_name, meta.args_json);
	state.record_tool_event(ToolEvent {
		turn:state.turn_counter,
		tool:tool_name.to_string(),
		sig,
		ok,
		error_sig,
		bytes:content.len(),
		wrote_path,
	});
}

/// Infer the written path for a write-style tool (P2/T6, feeds P11). Reads a
/// `path`/`file` arg for `write_file`/`patch`; falls back to the first-line
/// path heuristic used by `extract_file_path`.
fn wrote_path_from(tool_name:&str, args:Option<&serde_json::Value>, content:&str) -> Option<String> {
	if !matches!(tool_name, "write_file" | "patch") {
		return None;
	}
	if let Some(v) = args {
		for key in ["path", "file", "file_path", "filename"] {
			if let Some(p) = v.get(key).and_then(|x| x.as_str())
				&& !p.is_empty()
			{
				return Some(p.to_string());
			}
		}
	}
	extract_file_path(content, tool_name)
}

/// Resolve the reported `type` and the content the preview is built from.
///
/// Shared by the tool-result and terminal-output paths (F14): both resolve
/// an optional classify pair, falling back to Aphrodite's semantic detector
/// with a single terminal override, so the reported `type` AND the preview
/// both carry the high-signal shape. Detection stays in Aphrodite's layer -
/// the vendored classifier is untouched. Single type-resolution contract
/// (1.5.0 REFACTOR-PLAN §5).
///
/// `terminal_hint` enables the override: terminal output carrying an
/// `exit code:` / `Error:` line is always typed `terminal`, no matter what
/// the detector says. The tool path passes `false`, the terminal path
/// `true`.
fn resolve_type_and_classify_content<'a>(
	content:&'a str,
	classify:Option<(&'a str, &'a str)>,
	terminal_hint:bool,
) -> (String, &'a str) {
	match classify {
		Some((c, t)) => (t.to_string(), c),
		None => {
			let ct = transforms::content_detector::detect_content_type(content).content_type;
			let t = if terminal_hint && (content.contains("exit code:") || content.contains("Error:")) {
				"terminal".to_string()
			} else {
				// Terminal output is very often a git status / ls / test / grep
				// dump; upgrade a generic classification via Aphrodite's detector
				// so the preview is high-signal on the terminal path too.
				let base = ct.as_str().to_string();
				let inp = crate::preview::input::Input::new(content)
					.unwrap_or_else(|| crate::preview::input::Input::empty(content));
				crate::preview::r#type::resolve_effective_type(&base, &inp).into_owned()
			};
			(t, content)
		},
	}
}

fn transform_tool_result_inner(
	state:&mut AphroditeState,
	content:&str,
	tool_name:&str,
	classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
	// Record the telemetry event first (P2/T6): it must happen even when the
	// content is empty or the tool is essential/self/below-threshold - the
	// phase detector and error-loop breaker care about ALL calls, not only the
	// ones that produced a compressible marker. Skip only genuinely empty
	// content (no signal at all).
	if !content.is_empty() {
		record_tool_event_from_meta(state, tool_name, content, meta);
	}
	if content.is_empty() {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	}

	// Track file references BEFORE the essential-tool early return (F10):
	// `read_file` is both listed in `ESSENTIAL_TOOLS` (never compressed - the
	// agent needs the raw content) and in `state.file_tools` (tracked for
	// `aphrodite_files`/the catalog's `referenced_files` stat). Tracking used
	// to run after the essential-tool check returned early, so the single
	// most common file-reference source - reading a file - was silently
	// never recorded. Skipping compression and recording a reference are
	// independent decisions; do both regardless of which tools land in
	// which list.
	if state.file_tools.contains(&tool_name.to_string())
		&& let Some(path) = extract_file_path(content, tool_name)
	{
		state.record_file(path, tool_name.to_string());
	}

	// Skip essential tools
	if ESSENTIAL_TOOLS.contains(&tool_name) {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "essential_tool"});
	}

	// Skip aphrodite's own tools (and headroom helpers): their output is already
	// compact metadata, and compressing aphrodite_retrieve would replace the
	// resolved content with another CCR marker - an infinite preview loop.
	if tool_name.starts_with("aphrodite_") || tool_name.starts_with("headroom") {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "self_tool"});
	}

	// Skip below threshold (0 = always compress). Auto-expand bypasses this
	// gate: under auto-expand every non-essential tool result within the
	// limit goes through the store+expand path so no marker can leak.
	if !auto_expand_applies(state, content) && state.tool_threshold > 0 && content.len() < state.tool_threshold {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
	}

	// ── Live container (pre-Rust port of _hooks/live.py) ──
	// Enabled AND above the compress threshold → store the content chunked
	// with a metadata envelope and emit the INSTANT marker with a progress
	// annotation; the retrieve path serves the chunks progressively. When
	// auto-expand applies it wins (it guarantees no marker reaches the
	// model, so the container path is skipped and the expanded content is
	// returned inline exactly as today).
	if state.live_container && !auto_expand_applies(state, content) {
		let (type_str, _classify_content) = resolve_type_and_classify_content(content, classify, false);
		return transform_live_container(state, content, &type_str);
	}

	let (type_str, classify_content) = resolve_type_and_classify_content(content, classify, false);
	let hash = headroom_core::ccr::compute_key(content.as_bytes());

	state.inline_store_put(hash.clone(), content.to_string());

	let preview = crate::build_preview(&type_str, classify_content);
	let marker = ccr_marker(&hash, &type_str, content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:type_str.clone(),
		size:content.len(),
		preview:preview.clone(),
		turn:state.turn_counter,
		center:None,
		meta:None,
	});

	// Auto-expand (pre-Rust port): store + record exactly as above, but
	// return the RESOLVED content inline instead of the marker so no
	// `<<<CCR:...>>>` reaches the model. Over the limit or disabled, the
	// payload below is byte-identical to the pre-auto-expand shape.
	let expanded = if auto_expand_applies(state, content) {
		Some(auto_expand_content(state, &hash, content))
	} else {
		None
	};
	let mut payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": type_str,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	});
	if let Some(expanded) = expanded {
		payload["auto_expanded"] = serde_json::Value::Bool(true);
		payload["expanded"] = serde_json::Value::String(expanded);
	}
	payload
}

/// Transform terminal output - exit code aware.
pub fn transform_terminal_output(state:&mut AphroditeState, content:&str) -> serde_json::Value {
	transform_terminal_output_classified(state, content, None)
}

/// Same as [`transform_terminal_output`], with the same `classify` contract
/// as [`transform_tool_result_classified`].
pub fn transform_terminal_output_classified(
	state:&mut AphroditeState,
	content:&str,
	classify:Option<(&str, &str)>,
) -> serde_json::Value {
	transform_terminal_output_inner(state, content, classify, None, None)
}

/// Same as [`transform_terminal_output_classified`], plus the Hermes-supplied
/// `command` and `returncode` (report 05, P2/T6/T8) recorded into the
/// `tool_events` telemetry ring. `returncode == 0` (or absent) is `ok`.
pub fn transform_terminal_output_with_meta(
	state:&mut AphroditeState,
	content:&str,
	classify:Option<(&str, &str)>,
	command:Option<&str>,
	returncode:Option<i64>,
) -> serde_json::Value {
	transform_terminal_output_inner(state, content, classify, command, returncode)
}

fn transform_terminal_output_inner(
	state:&mut AphroditeState,
	content:&str,
	classify:Option<(&str, &str)>,
	command:Option<&str>,
	returncode:Option<i64>,
) -> serde_json::Value {
	// Telemetry (P2/T6): record before threshold/empty gating - a failing
	// command with tiny output is exactly what the error-loop breaker needs.
	if !content.is_empty() {
		let ok = returncode.map(|rc| rc == 0).unwrap_or(true);
		let args = command.map(|c| serde_json::json!({"command": c}));
		let sig = crate::flow::normalize_args_sig("terminal", args.as_ref());
		let error_sig = if ok {
			None
		} else {
			// Terminal failures have no error_type; key on the command + first
			// error-looking line of the output so distinct failures differ.
			let first_err = content
				.lines()
				.find(|l| l.contains("error") || l.contains("Error") || l.contains("FAILED") || l.contains("panicked"))
				.or_else(|| content.lines().next());
			Some(crate::flow::error_sig(command, first_err))
		};
		state.record_tool_event(ToolEvent {
			turn:state.turn_counter,
			tool:"terminal".to_string(),
			sig,
			ok,
			error_sig,
			bytes:content.len(),
			wrote_path:None,
		});
	}
	if content.is_empty() {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	}

	// ── Auto-expand gate (pre-Rust port) ──
	// Computed once, BEFORE the chain-split / threshold gates: when enabled
	// and the content fits the limit, no marker-emitting path may run at
	// all - the content goes straight to the store+expand single path so
	// no `<<<CCR:...>>>` (and no chain-split segment marker) reaches the
	// model. Over the limit or disabled, every gate below behaves exactly
	// as before.
	let do_auto_expand = auto_expand_applies(state, content);

	// ── Fine-grained chain splitting ──
	// Runs BEFORE the threshold gate: marked output must never leak raw
	// (below-threshold chained output would otherwise pass through with the
	// segment markers still in it, polluting what the LLM sees). Splitting
	// marked content is unconditional when the feature is enabled - except
	// under auto-expand (within the limit), where the whole content is
	// returned expanded instead.
	//
	// Invisibility contract: the JSON payload carries the per-segment
	// markers array (`markers`) - the Hermes bridge (replacement_from in
	// aphrodite-hermes) joins those markers into what the LLM sees, so it
	// reads exactly like any compressed output (N natural `<<<CCR:...>>>`
	// markers, each retrievable by its own hash). The `summary` string is
	// telemetry/metrics ONLY and must never be returned to the LLM - it
	// announces the mechanism.
	if state.chain_split_enabled && !do_auto_expand && content.contains(crate::chain_split::SEG_MARKER) {
		let parts = crate::chain_split::split_marked_output(content);
		if parts.len() >= 2 {
			let mut segments = Vec::with_capacity(parts.len());
			let mut total_orig = 0usize;
			let mut total_marker = 0usize;
			let mut seg_hashes = Vec::with_capacity(parts.len());
			for (idx, seg_text) in &parts {
				total_orig += seg_text.len();
				let ct = transforms::content_detector::detect_content_type(seg_text).content_type;
				let type_str = ct.as_str().to_string();
				let seg_hash = headroom_core::ccr::compute_key(seg_text.as_bytes());
				state.inline_store_put(seg_hash.clone(), seg_text.to_string());
				seg_hashes.push(seg_hash.clone());
				let mut preview = crate::build_preview(&type_str, seg_text);
				// Tier 3: per-segment error hint - content-level only. When
				// this segment's own output carries an error signal, surface
				// it in the preview so the LLM sees which segment failed and
				// why, without retrieving. Never mechanism vocabulary.
				// build-type previews already carry the first `error[...]`
				// message, so skip the hint there (no duplication). The hint
				// is merged INSIDE the preview's brackets: `[text:2L 63B |
				// scanning... ⚠ grep: ...]`, never appended after the `]`.
				if !preview.contains("error[")
					&& let Some(hint) = crate::chain_split::segment_error_hint(seg_text)
				{
					let inner = preview.trim_end_matches(']');
					preview = format!("{} ⚠ {}]", inner, hint);
				}
				let marker = ccr_marker(&seg_hash, &type_str, seg_text.len(), &preview, None, None, None);
				total_marker += marker.len();
				state.record_marker(MarkerEntry {
					hash:seg_hash.clone(),
					ccr_type:type_str.clone(),
					size:seg_text.len(),
					preview:preview.clone(),
					turn:state.turn_counter,
					center:None,
					meta:None,
				});
				segments.push(serde_json::json!({
					"index": idx,
					"type": type_str,
					"size": seg_text.len(),
					"hash": seg_hash,
					"preview": preview,
					"marker": marker,
				}));
			}
			// Tier 1 teaching loop: record the split event so segment
			// retrievals can be attributed and the split threshold adapted.
			state.record_chain_split(seg_hashes);
			let summary = format!(
				"[chain:{} segs | {} orig → {} markers]",
				segments.len(),
				total_orig,
				total_marker
			);
			return serde_json::json!({
				"status": "ok",
				"compressed": true,
				"chain_split": true,
				"segments": segments.len(),
				"summary": summary,
				"markers": segments,
			});
		}
	}

	// Threshold gate for UNMARKED output only: below-threshold content
	// passes through untouched (no markers to leak, nothing worth
	// splitting). Auto-expand bypasses the gate (store+expand path).
	if !do_auto_expand && state.terminal_threshold > 0 && content.len() < state.terminal_threshold {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
	}

	// ── Live container (pre-Rust port of _hooks/live.py) ──
	// Same contract as the tool path: enabled AND above the compress
	// threshold → chunked container store + INSTANT marker with a progress
	// annotation. Chain-split (above) already handled marked output, and
	// auto-expand wins when it applies.
	if state.live_container && !do_auto_expand {
		let (type_str, _classify_content) = resolve_type_and_classify_content(content, classify, true);
		return transform_live_container(state, content, &type_str);
	}

	let (type_str, classify_content) = resolve_type_and_classify_content(content, classify, true);

	let hash = headroom_core::ccr::compute_key(content.as_bytes());
	state.inline_store_put(hash.clone(), content.to_string());

	let preview = crate::build_preview(&type_str, classify_content);
	let marker = ccr_marker(&hash, &type_str, content.len(), &preview, None, None, None);

	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:type_str.to_string(),
		size:content.len(),
		preview:preview.clone(),
		turn:state.turn_counter,
		center:None,
		meta:None,
	});

	// Auto-expand (pre-Rust port): store + record exactly as above, but
	// return the RESOLVED content inline instead of the marker so no
	// `<<<CCR:...>>>` reaches the model. Over the limit or disabled, the
	// payload below is byte-identical to the pre-auto-expand shape.
	let expanded = if do_auto_expand {
		Some(auto_expand_content(state, &hash, content))
	} else {
		None
	};
	let mut payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": type_str,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	});
	if let Some(expanded) = expanded {
		payload["auto_expanded"] = serde_json::Value::Bool(true);
		payload["expanded"] = serde_json::Value::String(expanded);
	}
	payload
}

/// Session start hook - full reset.
pub fn on_session_start(state:&mut AphroditeState) -> serde_json::Value { crate::session::on_session_start(state) }

/// Pre-LLM call hook - inject catalog + active directives into context.
///
/// Keeps its historical JSON shape (`catalog`/`directives`/`compressed_count`)
/// for proxy/FFI consumers, but ALSO returns the unified `context` string built
/// by `flow::build_turn_context` (report 05, P1) so this path and the Hermes
/// bridge path can never fork on what the model actually sees. The bridge and
/// `context_engine_pre_llm` return only `{"context": ...}`; core keeps the
/// richer shape for back-compat.
pub fn pre_llm_call(state:&mut AphroditeState) -> serde_json::Value {
	// Poll-worker checkpoint: push nudges for running/completed/failed tasks
	// (only when the master flag is enabled).
	if state.poll_worker_enabled {
		crate::poll_worker::check_bg_tasks(state);
	}
	let directives = crate::directives::build_directive_context(&state.directives, &state.active_directives);
	// catalog_summary is now called inside build_turn_context - don't call
	// it separately (04-F3: double call skipped delta tracking every other turn).
	let context = crate::flow::build_turn_context(state, None);
	serde_json::json!({
		"status": "ok",
		"compressed_count": state.recent_markers.len(),
		"directives": if directives.is_empty() { None } else { Some(directives) },
		"context": if context.is_empty() { None } else { Some(context) },
	})
}

/// Post-LLM call hook - archive turn.
///
/// Archives the largest marker recorded this turn into `conv_index` before
/// advancing the turn counter (report 06 F11/T13/F13) - previously `archive_turn`
/// was never called from any hook, so `conv_index` stayed empty forever and
/// `aphrodite_diff` always returned zero turns despite compressions
/// happening every turn.
pub fn post_llm_call(state:&mut AphroditeState) -> serde_json::Value {
	// Archive the LARGEST marker recorded this turn (T13/F13): the last
	// recorded marker is usually the least informative (e.g. a tiny confirm
	// echo), while the largest carries the most content for the conversation
	// index. Ties keep the later-recorded entry.
	if let Some(largest) = state
		.recent_markers
		.iter()
		.filter(|m| m.turn == state.turn_counter)
		.max_by_key(|m| m.size)
	{
		let (hash, summary, size) = (largest.hash.clone(), largest.preview.clone(), largest.size);
		crate::session::archive_turn(state, &hash, &summary, size);
	}
	crate::session::next_turn(state);
	// P3/T9: purge expired ephemeral nudges AFTER advancing the turn, so a
	// one-shot pushed during turn N renders in turn N+1's context exactly once
	// and is gone by turn N+2.
	crate::flow::purge_expired_nudges(state);
	// Poll-worker: expire stale tasks (not polled in >STALE_TURN_AGE turns)
	// and drop completed/failed tasks older than 16 turns.
	crate::poll_worker::expire_stale_tasks(state);
	serde_json::json!({"status": "ok", "turn": state.turn_counter})
}

/// Extract file path from tool output - heuristic.
fn extract_file_path(content:&str, tool:&str) -> Option<String> {
	match tool {
		"read_file" | "write_file" | "patch" => {
			// First line often contains path
			content.lines().next().and_then(|line| {
				let line = line.trim();
				if line.starts_with('/') || line.starts_with("./") {
					Some(line.to_string())
				} else {
					None
				}
			})
		},
		// `search_files` output is conventionally a `path:line:match` (grep-
		// style) list; the file being referenced is whatever's before the
		// first `:` on the first result line (F10: this tool is listed in
		// `state.file_tools` but was previously never actually matched here,
		// so every search result silently went untracked).
		"search_files" => {
			content.lines().next().and_then(|line| {
				let path = line.split(':').next().unwrap_or("").trim();
				if path.starts_with('/') || path.starts_with("./") {
					Some(path.to_string())
				} else {
					None
				}
			})
		},
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_essential_tool_skip() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "some content", "skill_view");
		assert_eq!(r["compressed"], false);
		assert_eq!(r["reason"], "essential_tool");
	}

	// ── T9 (F10): `read_file` is both an essential tool (never compressed -
	// the agent needs the raw content) AND a tracked file tool
	// (`state.file_tools`); both must hold at once. Previously the essential-
	// tool early return ran before file-reference tracking, so the single
	// most common file reference (reading a file) was silently never
	// recorded in `referenced_files`.
	#[test]
	fn test_essential_tool_read_file_still_records_file_reference() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "/tmp/some/file.rs\nfn main() {}\n", "read_file");
		assert_eq!(r["compressed"], false, "read_file must never be compressed");
		assert_eq!(r["reason"], "essential_tool");
		assert_eq!(
			s.referenced_files.len(),
			1,
			"read_file must still be tracked as a file reference"
		);
		assert_eq!(s.referenced_files[0].0, "/tmp/some/file.rs");
	}

	// ── T9 (F10): `search_files` reaches the file-reference tracker (it's
	// listed in `state.file_tools`) but `extract_file_path` had no arm for
	// it, so search results were never recorded despite being tracked tools.
	#[test]
	fn test_search_files_records_file_reference() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 0; // always compress, to also exercise the non-essential path
		let content = "/tmp/some/file.rs:42:    let x = 1;\n/tmp/other/file.rs:7:    let y = 2;\n";
		let _ = transform_tool_result(&mut s, content, "search_files");
		assert_eq!(s.referenced_files.len(), 1);
		assert_eq!(s.referenced_files[0].0, "/tmp/some/file.rs");
	}

	#[test]
	fn test_empty_skip() {
		let mut s = AphroditeState::default();
		let r = transform_tool_result(&mut s, "", "terminal");
		assert_eq!(r["compressed"], false);
	}

	#[test]
	fn test_below_threshold() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 10000;
		let r = transform_tool_result(&mut s, "short", "terminal");
		assert_eq!(r["compressed"], false);
	}

	#[test]
	fn test_transform_success() {
		let mut s = AphroditeState::default();
		s.tool_threshold = 0; // always compress
		let content = "fn main() {\n    println!(\"hello world\");\n}\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["compressed"], true);
		let hash = r["hash"].as_str().unwrap();
		assert!(hash.len() >= 40);
		// The round-trip is the actual promise here: the marker this hook hands
		// back to the LLM must resolve, via aphrodite_retrieve, back to the
		// exact original content - not merely produce a hash-shaped string.
		let resolved = crate::resolve::expand(&mut s, hash);
		assert_eq!(resolved, Some(content.to_string()));
	}

	#[test]
	fn test_terminal_exit_code() {
		let mut s = AphroditeState::default();
		s.terminal_threshold = 0;
		let r = transform_terminal_output(&mut s, "error: broke\nexit code: 1\n");
		assert_eq!(r["type"], "terminal");
	}

	// ── T13 (F11): post_llm_call must archive the turn's last marker into
	// conv_index so aphrodite_diff returns non-zero results.
	#[test]
	fn test_post_llm_call_archives_turn() {
		let mut s = AphroditeState::default();
		s.turn_counter = 2;
		// Record a marker on the current turn.
		s.record_marker(MarkerEntry {
			hash:"hash42".into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text] summary".into(),
			turn:2,
			center:None,
			meta:None,
		});
		let _ = post_llm_call(&mut s);
		assert_eq!(s.turn_counter, 3);
		assert_eq!(s.conv_index.len(), 1, "post_llm_call must archive the turn");
		assert_eq!(s.conv_index[&2].0, "hash42");
	}

	// ── T13 (F13): when a turn records several markers, post_llm_call must
	// archive the LARGEST one, not merely the last recorded - the reverse
	// find picked the least informative marker (e.g. a tiny confirm echo
	// recorded after a big tool dump).
	#[test]
	fn test_post_llm_call_archives_largest_same_turn_marker() {
		let mut s = AphroditeState::default();
		s.turn_counter = 7;
		let record = |s:&mut AphroditeState, hash:&str, size:usize| {
			s.record_marker(MarkerEntry {
				hash:hash.into(),
				ccr_type:"text".into(),
				size,
				preview:format!("[text] {}", size),
				turn:7,
				center:None,
				meta:None,
			});
		};
		record(&mut s, "large", 900);
		// Two smaller markers recorded AFTER the large one - the old
		// reverse-find would have picked one of these.
		record(&mut s, "small-1", 40);
		record(&mut s, "small-2", 41);
		let _ = post_llm_call(&mut s);
		assert_eq!(s.turn_counter, 8);
		assert_eq!(s.conv_index.len(), 1, "post_llm_call must archive the turn");
		assert_eq!(s.conv_index[&7].0, "large", "largest same-turn marker must win");
	}

	// ── Poll-worker integration tests ──────────────────────────
	// Agent-agnostic tests: check_bg_tasks in pre_llm_call and
	// expire_stale_tasks in post_llm_call.  Agent-specific trigger
	// tests (auto-backgrounding from tool results) live in
	// aphrodite-hermes.

	#[test]
	fn test_pre_llm_call_pushes_poll_nudges() {
		let mut s = AphroditeState::default();
		s.turn_counter = 3;
		crate::poll_worker::insert_bg_task(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
		let _ = pre_llm_call(&mut s);
		// The nudge should have been pushed into ephemeral_directives.
		assert!(!s.ephemeral_directives.is_empty(), "pre_llm_call should push poll-worker nudge");
		let nudge = s.ephemeral_directives.last().unwrap();
		assert!(
			nudge.inline.as_deref().unwrap().contains("cargo build"),
			"nudge should mention the command"
		);
	}

	#[test]
	fn test_post_llm_call_expires_stale_poll_tasks() {
		let mut s = AphroditeState::default();
		s.turn_counter = 20;
		crate::poll_worker::insert_bg_task(
			&mut s,
			"old-task".into(),
			"terminal".into(),
			"old-build".into(),
			10, // started at turn 10, gap=10 < 16 so retain won't drop it
		);
		s.bg_tasks[0].last_poll_turn = 10; // never polled after start, gap=10 > 8
		let _ = post_llm_call(&mut s);
		// Should be marked stale.
		assert_eq!(
			s.bg_tasks[0].status,
			crate::poll_worker::BgStatus::Stale,
			"unpolled task should be expired as stale after {} turns",
			crate::poll_worker::STALE_TURN_AGE
		);
	}

	#[test]
	fn test_pre_llm_call_skips_nudges_when_poll_worker_disabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = false;
		s.turn_counter = 3;
		crate::poll_worker::insert_bg_task(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
		let before = s.ephemeral_directives.len();
		let _ = pre_llm_call(&mut s);
		assert_eq!(
			s.ephemeral_directives.len(),
			before,
			"disabled flag must prevent poll-worker nudges in pre_llm_call"
		);
	}

	#[test]
	fn test_post_llm_call_still_expires_when_poll_worker_disabled() {
		let mut s = AphroditeState::default();
		s.poll_worker_enabled = false;
		s.turn_counter = 20;
		crate::poll_worker::insert_bg_task(&mut s, "old-task".into(), "terminal".into(), "old-build".into(), 10);
		s.bg_tasks[0].last_poll_turn = 10;
		let _ = post_llm_call(&mut s);
		// Expiry is a cleanup concern - should still run even when disabled.
		assert_eq!(
			s.bg_tasks[0].status,
			crate::poll_worker::BgStatus::Stale,
			"expire_stale_tasks must still run when disabled (cleanup)"
		);
	}

	// ── Auto-expand (dylib-side consumer of `[compression] auto_expand`):
	// when enabled and content fits `auto_expand_limit`, the hook returns
	// the RESOLVED content inline (no `<<<CCR:...>>>` marker reaches the
	// model); over the limit or disabled, today's marker payload is
	// byte-identical. ──

	fn store_nested(state:&mut AphroditeState, text:&str) -> String {
		let h = compute_hash(text);
		state.inline_store_put(h.clone(), text.to_string());
		h
	}

	#[test]
	fn test_auto_expand_returns_expanded_content() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 102400;
		s.tool_threshold = 0;
		// A nested entry whose marker text is embedded in the tool output:
		// the resolved content must differ from the raw marker text, proving
		// the hook actually resolves instead of just passing through.
		let nested = "nested payload\nsecond line\n";
		let nested_hash = store_nested(&mut s, nested);
		let content = format!("<<<CCR:{nested_hash}|text|{}>>>", nested.len());
		let r = transform_tool_result(&mut s, &content, "terminal");
		assert_eq!(r["compressed"], true);
		assert_eq!(r["auto_expanded"], true, "auto-expand must flag the payload");
		assert_eq!(r["expanded"], nested, "expanded must be the resolved nested content");
		// The marker is still recorded and resolvable (retrieval stays
		// lossless). The stored content was a single wrapper marker, so
		// recursive expansion lands on the nested text itself.
		let content_hash = r["hash"].as_str().unwrap().to_string();
		assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
		assert_eq!(crate::resolve::expand(&mut s, &content_hash), Some(nested.to_string()));
		assert_eq!(crate::resolve::expand(&mut s, &nested_hash), Some(nested.to_string()));
	}

	#[test]
	fn test_auto_expand_terminal_returns_expanded_content() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 102400;
		s.terminal_threshold = 0;
		let nested = "terminal nested content";
		let nested_hash = store_nested(&mut s, nested);
		let content = format!("line 1\n<<<CCR:{nested_hash}|text|{}>>>\nline 3", nested.len());
		let r = transform_terminal_output(&mut s, &content);
		assert_eq!(r["compressed"], true);
		assert_eq!(r["auto_expanded"], true);
		assert_eq!(
			r["expanded"],
			format!("line 1\n{nested}\nline 3"),
			"expanded must resolve the nested marker inline"
		);
	}

	#[test]
	fn test_auto_expand_over_limit_emits_marker() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 8;
		s.tool_threshold = 0;
		let content = "this content is far longer than eight bytes";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["compressed"], true);
		assert!(
			r.get("auto_expanded").is_none(),
			"over-limit content must keep today's marker payload (no auto-expand fields)"
		);
		assert!(r.get("expanded").is_none());
		assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
		// The stored entry must still resolve to the exact original.
		let hash = r["hash"].as_str().unwrap();
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.to_string()));
	}

	#[test]
	fn test_auto_expand_terminal_over_limit_emits_marker() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 4;
		s.terminal_threshold = 0;
		let content = "way over the four byte limit";
		let r = transform_terminal_output(&mut s, content);
		assert_eq!(r["compressed"], true);
		assert!(r.get("auto_expanded").is_none(), "over-limit must emit the marker as today");
		assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
		let hash = r["hash"].as_str().unwrap();
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.to_string()));
	}

	// Default (auto_expand = false): the payload must be byte-identical to
	// the pre-auto-expand shape - same field set, same marker grammar.
	#[test]
	fn test_auto_expand_disabled_payload_is_byte_identical() {
		let mut s = AphroditeState::default(); // auto_expand = false
		s.tool_threshold = 0;
		let content = "fn main() {\n    println!(\"hello\");\n}\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["status"], "ok");
		assert_eq!(r["compressed"], true);
		assert!(r.get("auto_expanded").is_none());
		assert!(r.get("expanded").is_none());
		let hash = r["hash"].as_str().unwrap();
		assert_eq!(hash, &compute_hash(content));
		assert_eq!(r["size"], content.len() as u64);
		let mut keys:Vec<&str> = r.as_object().unwrap().keys().map(|k| k.as_str()).collect();
		let mut expected = vec!["status", "compressed", "hash", "type", "size", "preview", "marker"];
		keys.sort();
		expected.sort();
		assert_eq!(keys, expected, "default path must keep the exact pre-auto-expand field set");
		let expect_marker = format!(
			"<<<CCR:{}|{}|{}>>>\n{}",
			hash,
			r["type"].as_str().unwrap(),
			content.len(),
			r["preview"].as_str().unwrap()
		);
		assert_eq!(r["marker"], expect_marker, "default path must render the exact marker grammar");
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.to_string()));
	}

	#[test]
	fn test_auto_expand_disabled_terminal_payload_is_byte_identical() {
		let mut s = AphroditeState::default();
		s.terminal_threshold = 0;
		let content = "cargo build\n   Compiling aphrodite v0.1.0\nexit code: 0\n";
		let r = transform_terminal_output(&mut s, content);
		assert_eq!(r["compressed"], true);
		assert!(r.get("auto_expanded").is_none());
		assert!(r.get("expanded").is_none());
		let hash = r["hash"].as_str().unwrap();
		let expect_marker = format!(
			"<<<CCR:{}|{}|{}>>>\n{}",
			hash,
			r["type"].as_str().unwrap(),
			content.len(),
			r["preview"].as_str().unwrap()
		);
		assert_eq!(r["marker"], expect_marker);
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.to_string()));
	}

	// ── Gate interplay: auto-expand within the limit wins over chain-split
	// and the below-threshold pass-through, so no marker can leak. ──

	#[test]
	fn test_auto_expand_bypasses_chain_split_and_threshold() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 102400;
		s.chain_split_enabled = true;
		s.terminal_threshold = 100000; // content below the threshold
		let content = format!(
			"seg a output\n{}\nseg b output\n{}\nseg c output\n",
			crate::chain_split::SEG_MARKER,
			crate::chain_split::SEG_MARKER
		);
		let r = transform_terminal_output(&mut s, &content);
		assert_eq!(
			r["auto_expanded"], true,
			"auto-expand must win over chain-split within the limit"
		);
		assert!(r.get("chain_split").is_none(), "no chain-split payload under auto-expand");
		assert_eq!(
			r["expanded"], content,
			"expanded content must match the original (no nested markers)"
		);
	}

	#[test]
	fn test_chain_split_unaffected_when_auto_expand_off() {
		let mut s = AphroditeState::default(); // auto_expand = false
		s.chain_split_enabled = true;
		s.terminal_threshold = 0;
		let content = format!(
			"seg a output\n{}\nseg b output\n{}\nseg c output\n",
			crate::chain_split::SEG_MARKER,
			crate::chain_split::SEG_MARKER
		);
		let r = transform_terminal_output(&mut s, &content);
		assert_eq!(r["chain_split"], true, "chain-split must keep working when auto-expand is off");
	}

	#[test]
	fn test_auto_expand_bypasses_tool_threshold_gate() {
		let mut s = AphroditeState::default();
		s.auto_expand = true;
		s.auto_expand_limit = 102400;
		s.tool_threshold = 10000; // content below the threshold
		let content = "tiny tool output";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(
			r["auto_expanded"], true,
			"auto-expand must store+expand below-threshold content so no marker leaks"
		);
		assert_eq!(r["expanded"], content);
	}

	// ── Live containers (pre-Rust port of _hooks/live.py): enabled AND
	// above the compress threshold → chunked container store + INSTANT
	// marker with a progress annotation; off / below threshold → today's
	// behavior byte-identical. ──

	// Contract 1: `live_container` off (the default) must keep the payload
	// byte-identical to the pre-live shape - same field set, same marker
	// grammar, no container fields, no container store keys.
	#[test]
	fn test_live_container_off_payload_is_byte_identical() {
		let mut s = AphroditeState::default(); // live_container = false
		s.tool_threshold = 0;
		let content = "fn main() {\n    println!(\"hello\");\n}\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["status"], "ok");
		assert_eq!(r["compressed"], true);
		assert!(r.get("live").is_none(), "off must not emit container fields");
		let hash = r["hash"].as_str().unwrap();
		let expect_marker = format!(
			"<<<CCR:{}|{}|{}>>>\n{}",
			hash,
			r["type"].as_str().unwrap(),
			content.len(),
			r["preview"].as_str().unwrap()
		);
		assert_eq!(r["marker"], expect_marker, "off must keep the exact marker grammar");
		assert!(
			!s.inline_store.contains_key(&live_meta_key(hash)),
			"off must not store a container envelope"
		);
		assert!(
			!s.inline_store.contains_key(&live_chunk_key(hash, 0)),
			"off must not store container chunks"
		);
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.to_string()));
	}

	// Contract 2: enabled but below the compress threshold → unchanged
	// (no container) on BOTH the tool and terminal paths.
	#[test]
	fn test_live_container_below_threshold_unchanged() {
		let mut s = AphroditeState::default();
		s.live_container = true;
		s.tool_threshold = 10_000;
		let r = transform_tool_result(&mut s, "short output", "terminal");
		assert_eq!(r["compressed"], false);
		assert_eq!(r["reason"], "below_threshold");
		assert!(r.get("live").is_none(), "below-threshold must not emit a container");

		let mut s2 = AphroditeState::default();
		s2.live_container = true;
		s2.terminal_threshold = 10_000;
		let r2 = transform_terminal_output(&mut s2, "tiny terminal output");
		assert_eq!(r2["compressed"], false);
		assert_eq!(r2["reason"], "below_threshold");
		assert!(r2.get("live").is_none());
	}

	// Contract 3: enabled + over the threshold → INSTANT marker with a
	// progress annotation; the store carries the container (meta envelope +
	// per-chunk entries) AND the round-trip hash; the marker's hash still
	// resolves to the exact original.
	#[test]
	fn test_live_container_tool_over_threshold_instant_marker_with_progress() {
		let mut s = AphroditeState::default();
		s.live_container = true;
		s.tool_threshold = 0; // always compress
		let content = (0..40)
			.map(|i| format!("tool payload line {i:02} with some text\n"))
			.collect::<String>();
		let chunks = chunk_on_char_boundaries(&content, LIVE_CHUNK_SIZE);
		assert!(chunks.len() >= 2, "test content must span multiple chunks");
		let r = transform_tool_result(&mut s, &content, "terminal");
		assert_eq!(r["status"], "ok");
		assert_eq!(r["compressed"], true);
		assert_eq!(r["live"], true);
		assert_eq!(r["type"], "live", "the marker type must be `live`");
		assert_eq!(r["size"].as_u64().unwrap() as usize, content.len());
		assert_eq!(r["chunks"].as_u64().unwrap() as usize, chunks.len());
		assert_eq!(r["bytes_arrived"], 0, "the instant marker reports zero bytes arrived");
		let hash = r["hash"].as_str().unwrap();
		let marker = r["marker"].as_str().unwrap();
		assert!(
			marker.starts_with(&format!("<<<CCR:{}|live|{}>>>", hash, content.len())),
			"marker must use the live grammar: {marker}"
		);
		assert!(marker.contains("Live container"), "marker must carry the annotation: {marker}");
		assert!(
			marker.contains(&format!("{} chunks", chunks.len())),
			"marker must carry the chunk count: {marker}"
		);
		assert!(
			marker.contains("0 bytes arrived"),
			"marker must carry the progress annotation: {marker}"
		);
		// Container store: meta envelope + per-chunk entries + round-trip hash.
		assert!(
			s.inline_store.contains_key(&live_meta_key(hash)),
			"meta envelope must be stored"
		);
		for i in 0..chunks.len() {
			assert!(
				s.inline_store.contains_key(&live_chunk_key(hash, i)),
				"chunk {i} must be stored"
			);
		}
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content.clone()));
		assert!(
			s.recent_markers.iter().any(|m| m.hash == hash && m.ccr_type == "live"),
			"the live entry must be recorded in the catalog"
		);
	}

	// Contract 3 (terminal path): same instant-marker contract through
	// `transform_terminal_output`.
	#[test]
	fn test_live_container_terminal_over_threshold_instant_marker() {
		let mut s = AphroditeState::default();
		s.live_container = true;
		s.terminal_threshold = 0;
		let content = (0..40)
			.map(|i| format!("cargo step {i:02} done - line content\n"))
			.collect::<String>();
		let r = transform_terminal_output(&mut s, &content);
		assert_eq!(r["live"], true);
		assert_eq!(r["type"], "live");
		assert_eq!(
			r["chunks"].as_u64().unwrap() as usize,
			chunk_on_char_boundaries(&content, LIVE_CHUNK_SIZE).len()
		);
		let hash = r["hash"].as_str().unwrap();
		assert!(
			r["marker"]
				.as_str()
				.unwrap()
				.starts_with(&format!("<<<CCR:{}|live|{}>>>", hash, content.len()))
		);
		assert_eq!(crate::resolve::expand(&mut s, hash), Some(content));
	}

	// Contract 4: the default state keeps `live_container` off - no config
	// change, no behavioral drift.
	#[test]
	fn test_live_container_default_state_unchanged() {
		let s = AphroditeState::default();
		assert_eq!(s.live_container, false, "default state must keep live_container off");
	}

	// Auto-expand wins over the container path: under auto-expand (within
	// the limit) the hook returns the expanded content inline even when
	// live_container is on - no container marker may leak.
	#[test]
	fn test_live_container_defers_to_auto_expand_within_limit() {
		let mut s = AphroditeState::default();
		s.live_container = true;
		s.auto_expand = true;
		s.auto_expand_limit = 102400;
		s.tool_threshold = 0;
		let content = "some content that would otherwise become a live container\n";
		let r = transform_tool_result(&mut s, content, "terminal");
		assert_eq!(r["auto_expanded"], true, "auto-expand must win over the container path");
		assert!(r.get("live").is_none());
		assert_eq!(r["expanded"], content);
	}
}
