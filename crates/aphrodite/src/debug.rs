//! DEV-ONLY env-gated debug logger (TEMPORARY development instrumentation).
//!
//! WARNING: this module is temporary dev-only compute. It is gated behind
//! `APHRODITE_DEBUG` (read into `AphroditeState::debug_level` by
//! `config_loader::apply_compression` at session start) and must NEVER be
//! relied on in a release path. When the level is 0 every call site is a
//! no-op: the enabled check is one relaxed atomic load and no formatting,
//! file I/O, or timing runs (byte-identical behavior - existing tests stay
//! green).
//!
//! When enabled (1 = on, 2 = verbose), timestamped single-line structured
//! records are APPENDED to `debug.<root-session-id>` under the runtime home
//! (`~/.hermes/aphrodite/`; $APHRODITE_HOME -> $HERMES_HOME -> $HOME ->
//! platform default - the same `home::runtime_home` decision the flag-file
//! toggle in `aphrodite-hermes/src/debug.rs` uses). The root session id is
//! published by the Hermes bridge (`note_root_session`, fed from its
//! `record_session`) and persisted across processes via the `session.current`
//! file, so a fresh Hermes process still targets the same per-session log.
//!
//! The `debug.<root>` file doubles as the legacy flag file: the flag reader
//! (`aphrodite-hermes` `flag_enabled`) resolves the FIRST line only, so an
//! appended log history never flips the toggle, and a toggle write ("on" /
//! "off", whole-file overwrite) still reads back correctly. Appended lines
//! are the logger channel; the first line decides the flag.
//!
//! Every write is best-effort: a failed append (missing dir, permissions)
//! degrades to silence - a debug-logging failure must never break a
//! compression or turn.

use std::{
	path::PathBuf,
	sync::{
		Mutex,
		OnceLock,
		atomic::{AtomicU8, AtomicU64, Ordering},
	},
};

use crate::state::AphroditeState;

/// Process-global debug level mirror (0/1/2), set from
/// `AphroditeState::debug_level` at config load so stateless render points
/// (templates, marker format) gate on the same flag.
static LEVEL:AtomicU8 = AtomicU8::new(0);

/// The most recently seen ROOT session id (published by the Hermes bridge's
/// `record_session`). Persisted across process restarts via the
/// `session.current` file, which the bridge also writes.
static ROOT_SESSION:OnceLock<Mutex<String>> = OnceLock::new();

/// DEV-ONLY cumulative counters (process-global; the dylib hosts one
/// session per process). Indices must match [`Counter`].
static COUNTERS:[AtomicU64; 10] = [const { AtomicU64::new(0) }; 10];

/// DEV-ONLY timing accumulators (sum + sample count per [`TimingCat`]).
static TIMING_SUM:[AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static TIMING_N:[AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];

/// Cumulative counter kinds tracked by the debug logger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counter {
	/// Markers recorded via `AphroditeState::record_marker` (every path).
	MarkersCreated = 0,
	/// Compress decisions that stored content (tool/terminal/chain-split).
	Compressions = 1,
	/// Auto-expand resolutions (content returned inline instead of a marker).
	AutoExpansions = 2,
	/// Live-container wraps.
	LiveContainers = 3,
	/// Context-engine middle-turn archives.
	EngineArchives = 4,
	/// Top-level retrieval requests (`resolve::expand`).
	Retrieves = 5,
	/// Retrievals that missed (hash not found).
	RetrieveMisses = 6,
	/// Preview/marker template renders (TOML `[templates]` hits).
	TemplateRenders = 7,
	/// Tool results reported with an error status.
	ToolErrors = 8,
	/// Degraded outcomes (retrieval misses, ...) worth counting.
	Warnings = 9,
}

/// Timing categories averaged in the metrics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingCat {
	/// Tool/terminal transform hook duration.
	ToolTransform = 0,
	/// Retrieval (`resolve::expand`) duration.
	Retrieve = 1,
	/// `flow::build_turn_context` duration.
	PreTurn = 2,
	/// Template render duration.
	Render = 3,
}

/// Set the process-global debug level (0/1/2). Called from
/// `config_loader::apply_compression` so the env var captured at session
/// start reaches every stateless render point (templates, marker format).
pub fn set_level(level:u8) { LEVEL.store(level.min(2), Ordering::Relaxed); }

/// The current debug level - the cheap gate (one relaxed atomic load).
#[inline]
pub fn level() -> u8 { LEVEL.load(Ordering::Relaxed) }

/// Cheap gate: true only when the debug logger is enabled.
#[inline]
pub fn enabled() -> bool { level() != 0 }

/// Publish the root session id (Hermes bridge `record_session`).
pub fn note_root_session(root:&str) {
	if root.is_empty() {
		return;
	}
	let mut guard = ROOT_SESSION
		.get_or_init(|| Mutex::new(String::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	if guard.is_empty() || *guard != root {
		*guard = root.to_string();
	}
}

/// Resolve the current root session id: the in-process static, falling back
/// to the `session.current` file the bridge persists across restarts.
pub fn current_root_session() -> String {
	{
		let guard = ROOT_SESSION
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		if !guard.is_empty() {
			return guard.clone();
		}
	}
	std::fs::read_to_string(debug_runtime_home().join("session.current"))
		.ok()
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

/// The debug runtime home: $APHRODITE_DEBUG_HOME (dev-only override, used
/// by tests to keep probe files out of the real runtime home) -> the real
/// runtime home in production -> a scratch dir under the OS temp in tests.
/// The scratch fallback matters only when this crate is the test crate
/// itself; as a dependency it compiles WITHOUT cfg(test), so test isolation
/// relies on the env override (set by the hermes test suite).
fn debug_runtime_home() -> PathBuf {
	if let Ok(home) = std::env::var("APHRODITE_DEBUG_HOME")
		&& !home.is_empty()
	{
		return PathBuf::from(home);
	}
	#[cfg(test)]
	{
		let scratch = std::env::temp_dir().join("aphrodite-test-debug-home");
		let _ = std::fs::create_dir_all(&scratch);
		scratch
	}
	#[cfg(not(test))]
	{
		crate::home::runtime_home()
	}
}

/// The per-session debug log path; `None` until a root session is known
/// (the bridge publishes it on the first `pre_llm_call`).
fn log_path() -> Option<PathBuf> {
	let root = current_root_session();
	if root.is_empty() {
		return None;
	}
	Some(log_path_for_root(&root))
}

/// DEV-ONLY: the log file path for a root session (test/visibility helper -
/// honors the test-home isolation).
pub fn log_path_for_root(root:&str) -> PathBuf { debug_runtime_home().join(format!("debug.{root}")) }

/// Sanitize a kv value for the single-line structured format.
pub(crate) fn sanitize_kv(value:&str) -> String { value.replace(['\n', '\r'], " ") }

/// Append one timestamped structured line to `debug.<root>`. Best-effort:
/// any failure (missing dir, permissions) degrades to silence - never a
/// panic, never a broken turn.
pub fn log_at(level:u8, event:&str, kv:&[(&str, String)]) {
	if level == 0 {
		return;
	}
	let Some(path) = log_path() else { return };
	let mut line = format!("[{}] [aphrodite-debug] lvl={level} event={event}", iso_timestamp());
	for (key, value) in kv {
		line.push_str(&format!(" {key}={}", sanitize_kv(value)));
	}
	line.push('\n');
	// Fresh-line guarantee: a legacy flag file may lack a trailing newline
	// (the toggle writes "on"/"off" with no terminator), so a raw append
	// would merge the first log line into the flag value and flip the flag
	// off. Prefix a newline when the file does not already end with one.
	let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
		Ok(f) => f,
		// Best-effort: any failure (missing dir, permissions) degrades to
		// silence - a debug-logging failure must never break a turn.
		Err(_) => return,
	};
	use std::io::{Read, Seek, SeekFrom, Write};
	if f.seek(SeekFrom::End(-1)).is_ok() {
		let mut last = [0u8; 1];
		if f.read_exact(&mut last).is_ok() && last[0] != b'\n' {
			let _ = f.write_all(b"\n");
		}
	}
	let _ = f.write_all(line.as_bytes());
}

/// Log at the process-global level (stateless call sites).
pub fn log(event:&str, kv:&[(&str, String)]) { log_at(level(), event, kv); }

/// Verbose-only line (level >= 2).
pub fn log_verbose(event:&str, kv:&[(&str, String)]) {
	if level() >= 2 {
		log_at(2, event, kv);
	}
}

/// Increment a cumulative counter. No-op when the logger is off (one
/// relaxed load on the quiet path).
pub fn bump(counter:Counter) {
	if !enabled() {
		return;
	}
	COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Add a timing sample (no-op when the logger is off).
pub fn add_timing(cat:TimingCat, us:u64) {
	if !enabled() {
		return;
	}
	let idx = cat as usize;
	TIMING_SUM[idx].fetch_add(us, Ordering::Relaxed);
	TIMING_N[idx].fetch_add(1, Ordering::Relaxed);
}

/// DEV-ONLY: engine skip line - the context-engine decision when nothing
/// was archived this turn.
pub fn engine_skip(state:&AphroditeState, reason:&str) {
	if state.debug_level == 0 {
		return;
	}
	log_at(state.debug_level, "engine_skip", &[("reason", reason.to_string())]);
}

/// Snapshot the cumulative counters + timing averages as JSON (emitted on
/// each pre-turn line and surfaced in `aphrodite_stats`).
pub fn metrics_json() -> serde_json::Value {
	let c = |idx:usize| COUNTERS[idx].load(Ordering::Relaxed);
	let avg = |sum:&AtomicU64, n:&AtomicU64| -> f64 {
		let n = n.load(Ordering::Relaxed);
		if n == 0 { 0.0 } else { sum.load(Ordering::Relaxed) as f64 / n as f64 }
	};
	serde_json::json!({
		"markers_created": c(Counter::MarkersCreated as usize),
		"compressions": c(Counter::Compressions as usize),
		"auto_expansions": c(Counter::AutoExpansions as usize),
		"live_containers": c(Counter::LiveContainers as usize),
		"engine_archives": c(Counter::EngineArchives as usize),
		"retrieves": c(Counter::Retrieves as usize),
		"retrieve_misses": c(Counter::RetrieveMisses as usize),
		"template_renders": c(Counter::TemplateRenders as usize),
		"tool_errors": c(Counter::ToolErrors as usize),
		"warnings": c(Counter::Warnings as usize),
		"timing": {
			"tool_transform_avg_us": avg(&TIMING_SUM[TimingCat::ToolTransform as usize], &TIMING_N[TimingCat::ToolTransform as usize]),
			"retrieve_avg_us": avg(&TIMING_SUM[TimingCat::Retrieve as usize], &TIMING_N[TimingCat::Retrieve as usize]),
			"pre_turn_avg_us": avg(&TIMING_SUM[TimingCat::PreTurn as usize], &TIMING_N[TimingCat::PreTurn as usize]),
			"render_avg_us": avg(&TIMING_SUM[TimingCat::Render as usize], &TIMING_N[TimingCat::Render as usize]),
		},
	})
}

/// DEV-ONLY: config-summary kv pairs for the session-init line.
pub fn config_summary(state:&AphroditeState) -> Vec<(&'static str, String)> {
	vec![
		("tool_threshold", state.tool_threshold.to_string()),
		("terminal_threshold", state.terminal_threshold.to_string()),
		("inline_byte_budget", state.inline_store_byte_budget().to_string()),
		("engine_threshold_pct", state.engine_threshold_pct.to_string()),
		("engine_min_msgs", state.engine_min_msgs.to_string()),
		("engine_protect_first", state.engine_protect_first.to_string()),
		("engine_protect_last", state.engine_protect_last.to_string()),
		("auto_expand", state.auto_expand.to_string()),
		("auto_expand_limit", state.auto_expand_limit.to_string()),
		("live_container", state.live_container.to_string()),
		("recursive_depth", state.recursive_depth.to_string()),
		("chain_split", state.chain_split_enabled.to_string()),
		("flow_budget_chars", state.flow_budget_chars.to_string()),
		("directives", state.active_directives.join(",")),
		(
			"templates",
			if state.templates.is_some() {
				"registered".to_string()
			} else {
				"builtin".to_string()
			},
		),
		("debug_level", state.debug_level.to_string()),
	]
}

/// Readable UTC timestamp `YYYY-MM-DDTHH:MM:SS.ffffff` (no chrono dep - this
/// is dev-only instrumentation; a hand-rolled civil-from-days conversion
/// keeps the dependency tree untouched).
fn iso_timestamp() -> String {
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default();
	let secs = now.as_secs();
	let micros = now.subsec_micros();
	let days = secs / 86_400;
	let rem = secs % 86_400;
	let (hh, mm, ss) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
	// Howard Hinnant's civil_from_days: days -> (y, m, d) in the proleptic
	// Gregorian calendar.
	let z = days as i64 + 719_468;
	let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
	let doe = z - era * 146_097; // [0, 146096]
	let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
	let y = yoe + era * 400;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
	let mp = (5 * doy + 2) / 153; // [0, 11]
	let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
	let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
	let y = (if m <= 2 { y + 1 } else { y }) as u32;
	format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{micros:06}Z")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_iso_timestamp_shape() {
		let s = iso_timestamp();
		// YYYY-MM-DDTHH:MM:SS.ffffffZ
		assert_eq!(s.len(), 27, "timestamp: {s}");
		assert!(s.ends_with('Z'));
		assert!(s.contains('T'));
		assert!(s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' && s.as_bytes()[10] == b'T');
	}

	#[test]
	fn test_level_gate_off_is_quiet() {
		set_level(0);
		assert!(!enabled());
		// level 0 must short-circuit before any file I/O: log_at(0, ...)
		// returns without touching the runtime home.
		log_at(0, "test_off", &[("k", "v".to_string())]);
	}

	#[test]
	fn test_sanitize_kv_flattens_newlines() {
		assert_eq!(sanitize_kv("a\nb\r c"), "a b  c");
	}

	#[test]
	fn test_counters_reset_by_default() {
		set_level(0);
		let m = metrics_json();
		assert_eq!(m["markers_created"], 0);
		assert_eq!(m["retrieves"], 0);
	}
}
