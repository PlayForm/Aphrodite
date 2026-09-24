//! Aphrodite internal state - mirrors plugins/aphrodite/_core/state.py
//! All session-scoped state lives here: inline store, conv index, markers,
//! counters.
//!
//! Atomized into a per-domain tree: `store.rs` (inline store + byte
//! budget), `markers.rs` (marker catalog), `events.rs` (tool-event ring),
//! `files.rs` (referenced files), `chain_split.rs` (Tier 1 teaching loop),
//! `directives.rs` (ephemeral directive activations), `tests.rs` (battery).
//! This facade re-exports the exact public names the rest of the crate
//! (and `aphrodite-hermes` via `lib.rs`) depends on.

use std::collections::{HashMap, VecDeque};

pub mod chain_split;
pub mod directives;
pub mod events;
pub mod files;
pub mod markers;
pub mod store;

#[cfg(test)]
mod tests;

pub use chain_split::SplitEvent;
pub use directives::ActiveDirective;
pub use events::ToolEvent;
pub use markers::MarkerEntry;

/// Maximum inline store entries before LRU eviction.
const INLINE_MAX:usize = 500;

/// Default byte budget for the inline store (report 05 F11): entry-count
/// alone (`INLINE_MAX`) doesn't bound memory - `aphrodite_prefetch` admits
/// files up to 10MB each and the ABI admits blobs up to 16MB, so 500 entries
/// at the large end is a multi-GB worst case with zero byte accounting.
/// 256MB is a conservative default for a single agent session's compression
/// cache; exposed via `AphroditeState::inline_store_byte_budget` so a config
/// layer can override it.
pub const DEFAULT_INLINE_BYTE_BUDGET:usize = 256 * 1024 * 1024;

/// Cap on the chain-split consequence ledger (events kept for adaptation).
const SPLIT_EVENT_CAP:usize = 16;

/// Cap on the per-tool-call telemetry ring (P2); oldest evicted from the
/// front.
const TOOL_EVENT_CAP:usize = 200;
/// Cap on the recent-marker catalog; oldest evicted from the front.
const RECENT_MARKERS_CAP:usize = 200;
/// Cap on the referenced-files list; least-recently-referenced evicted.
const REFERENCED_FILES_CAP:usize = 100;

/// Minimum split events recorded before the threshold adapts - avoids
/// adapting on noise from a single split.
const SPLIT_ADAPT_MIN_EVENTS:usize = 4;

/// Session state - one per loaded dylib instance.
pub struct AphroditeState {
	/// Inline content store: {hash: content}. The `HashMap` gives O(1)
	/// get/put/contains (was an O(n) `VecDeque` linear scan per op - bug
	/// 18-P14); `inline_order` preserves LRU recency so eviction can drop
	/// the least-recently-used entry in O(1).
	pub inline_store:HashMap<String, String>,
	/// LRU recency order of `inline_store` keys (unit values). Capacity is
	/// `INLINE_MAX + 1` so the cache never self-evicts before the explicit
	/// `evict_over_budget` loop in `inline_store_put` (fix 1: previously a
	/// `VecDeque` whose promotion/upsert needed an O(n) `retain` scan).
	pub inline_order:lru::LruCache<String, ()>,
	/// Running total of `content.len()` across every entry in `inline_store`,
	/// maintained incrementally by `inline_store_put` so eviction doesn't
	/// need an O(n) rescan on every insert (report 05 F11).
	inline_store_bytes:usize,
	/// Byte budget for `inline_store`; entries are evicted from the back
	/// (oldest/least-recently-used) until the running total is at or under
	/// this, in addition to the existing `INLINE_MAX` entry-count cap.
	/// Defaults to [`DEFAULT_INLINE_BYTE_BUDGET`]; see
	/// `inline_store_byte_budget`/`set_inline_store_byte_budget`.
	inline_store_byte_budget:usize,
	/// Recent CCR markers for catalog: [{hash, type, size, preview, turn}].
	/// `VecDeque` (fix 6): `record_marker` pushes back and pops the front
	/// while over `RECENT_MARKERS_CAP`, so the oldest marker always sits at
	/// index 0.
	pub recent_markers:VecDeque<MarkerEntry>,
	/// Conversation index: {turn_num: (hash, summary, size)} - the last
	/// marker archived per turn by `session::archive_turn`, called from
	/// `hooks::post_llm_call` (report 06 F11/T13: previously `archive_turn`
	/// was never called from any hook, so this stayed empty forever and
	/// `aphrodite_diff` always returned zero turns).
	pub conv_index:HashMap<usize, (String, String, usize)>,
	/// Referenced files: {filepath: last_tool_name}
	pub referenced_files:VecDeque<(String, String)>,
	/// Turn counter.
	pub turn_counter:usize,
	/// Scanned message index for incremental marker scan.
	pub scanned_msg_idx:usize,
	/// File tools set.
	pub file_tools:Vec<String>,
	// ── Config values (mirrored from aphrodite.toml) ──
	pub api_url:String,
	pub model:String,
	pub engine_threshold_pct:u64,
	// RESERVED: write-only today (loaded from aphrodite.toml, never read back
	// by the proxy) - candidate consumers for the context-engine work
	// (13-P2), not deleted since that work may land on them directly
	// (01-F9, user decision: keep-reserved over delete).
	pub engine_min_msgs:usize,
	pub engine_protect_first:usize,
	pub engine_protect_last:usize,
	pub context_engine_enabled:bool,
	pub tool_threshold:usize,
	pub terminal_threshold:usize,
	/// Parse-failure self-diagnosis: set when the loaded aphrodite.toml was
	/// found but failed to parse (defaults in effect). Surfaced verbatim in
	/// `aphrodite_stats` so a broken config is never indistinguishable from
	/// "config not set" - the tracing warn is a no-op in the Hermes dylib.
	pub config_error:Option<String>,
	/// Opt-in config auto-reload: watch aphrodite.toml and re-apply config
	/// fields on change (default off). Only config fields are mutated -
	/// session CCR state, directive selection, and telemetry are preserved.
	pub auto_reload:bool,
	// RESERVED: same as engine_min_msgs above (01-F9).
	pub catalog_mode:String,
	pub expand_guidance:bool,
	pub dev_mode:bool,
	// ── Conversational Directives ──
	/// All loaded directives (name → content).
	pub directives:std::collections::HashMap<String, crate::directives::Directive>,
	/// Currently active directive names (the ones injected into context).
	pub active_directives:Vec<String>,
	/// Ephemeral (one-shot / TTL) directives - inline nudges that render once
	/// (or for a bounded number of turns) then self-purge (P3/T9). Distinct
	/// from `active_directives` (permanent-until-removed named entries).
	pub ephemeral_directives:Vec<ActiveDirective>,
	// ── Flow context assembler (P1) ──
	/// Hard cap for ALL per-turn injected context assembled by
	/// `flow::build_turn_context` (default 4000 chars, `[flow] budget_chars`).
	pub flow_budget_chars:usize,
	/// First-turn session instruction loaded from `[prompts] session_inject`
	/// in aphrodite.toml - rendered once via `build_first_turn_injection`,
	/// then dropped (turn_counter > 0). Empty = no injection (default).
	pub session_inject:String,
	/// Turn number of the most recent MANUAL `aphrodite_directive` mutation
	/// (swap/add/remove/reset). Latches phase-aware auto-swaps out (P6); set by
	/// `directives::handle_action` on any successful mutation.
	pub manual_directive_turn:Option<usize>,
	// ── Turn-telemetry spine (P2) ──
	/// Bounded ring of per-tool-call events (cap 200, evict front). Feeds phase
	/// detection, error-loop breaking, delta previews, checkpoints (P6-P11).
	pub tool_events:VecDeque<ToolEvent>,
	// ── Poll-worker auto-backgrounding ──
	/// Background tasks created by the poll-worker auto-backgrounding
	/// heuristic (cap 4, evict oldest completed/stale on overflow).
	pub bg_tasks:VecDeque<crate::poll_worker::BgTask>,
	/// Master on/off for poll-worker auto-backgrounding. When false,
	/// no tool output is auto-backgrounded (existing bg_tasks still
	/// receive lifecycle nudges and expiry). Default true. Env:
	/// `APHRODITE_POLL_WORKER`, TOML: `[compression] poll_worker`.
	pub poll_worker_enabled:bool,
	/// Fine-grained chain splitting: rewrite chained shell commands
	/// (`a && b && c`) with segment markers and split the output into
	/// per-segment CCR entries, so the agent sees N compact previews
	/// instead of one giant blob. Struct default false (fix 2), matching
	/// the shipped config default; `apply_compression` resolves the TOML/env
	/// override (opt-in per session via `APHRODITE_CHAIN_SPLIT=1` or TOML
	/// `[compression] chain_split`). Env: `APHRODITE_CHAIN_SPLIT`, TOML:
	/// `[compression] chain_split`.
	pub chain_split_enabled:bool,
	// ── Tier 1 teaching loop: adaptive split threshold ──
	/// Current minimum segment count for chain splitting. Only chains with
	/// at least this many segments are rewritten. Starts at the configured
	/// floor (default 2, `split_chain` already requires ≥2) and adapts
	/// within `[chain_split_floor, chain_split_max_segments]` based on
	/// whether the agent actually retrieves the produced segment markers
	/// (consequence-driven learning). The threshold is machinery - never
	/// observable to the LLM (no directive text, no summary changes).
	pub chain_split_min_segments:usize,
	/// Lower bound of the adaptive threshold: the configured initial value.
	pub chain_split_floor:usize,
	/// Upper bound of the adaptive threshold. Env:
	/// `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS`, TOML:
	/// `[compression] chain_split_max_segments`. Default 6.
	pub chain_split_max_segments:usize,
	/// Consequence ledger: one entry per chain-split event recording how
	/// many segment markers were produced and how many distinct ones were
	/// later retrieved. Bounded ring (cap `SPLIT_EVENT_CAP`); oldest
	/// evicted. `split_segment_map` maps each produced segment hash to its
	/// event id so a resolve can be attributed to the right event.
	pub split_events:VecDeque<SplitEvent>,
	/// Segment hash → split-event id, for retrieval attribution. A hash
	/// resolves at most once (removed after counting), so re-retrieving the
	/// same marker never double-counts.
	pub split_segment_map:HashMap<String, usize>,
	/// Monotonic event id counter for `split_events`.
	pub split_next_event_id:usize,
	// ── Delta catalog (04-F1) ──
	/// Number of markers the last time catalog_summary rendered, so we emit a
	/// delta line only when new markers arrived this turn. Zero-initialized;
	/// reset on session start. Stops the prompt-cache-poisoning repetition of
	/// the same 5 previews every turn.
	pub last_emitted_marker_count:usize,
	/// Number of referenced files the last time catalog_summary rendered,
	/// for delta-only file listing (04-F4: stops re-listing same 5 files).
	pub last_emitted_file_count:usize,
}

impl Default for AphroditeState {
	fn default() -> Self {
		Self {
			inline_store:HashMap::with_capacity(INLINE_MAX),
			// +1 headroom: the LruCache must never self-evict before the
			// explicit evict loop enforces INLINE_MAX and the byte budget.
			inline_order:lru::LruCache::new(std::num::NonZeroUsize::new(INLINE_MAX + 1).unwrap()),
			inline_store_bytes:0,
			inline_store_byte_budget:DEFAULT_INLINE_BYTE_BUDGET,
			recent_markers:VecDeque::new(),
			conv_index:HashMap::new(),
			referenced_files:VecDeque::new(),
			turn_counter:0,
			scanned_msg_idx:0,
			file_tools:vec!["read_file".into(), "write_file".into(), "patch".into(), "search_files".into()],
			api_url:String::new(),
			model:"gpt-4o".into(),
			engine_threshold_pct:45,
			engine_min_msgs:8,
			engine_protect_first:2,
			engine_protect_last:5,
			context_engine_enabled:true,
			tool_threshold:512,
			terminal_threshold:256,
			config_error:None,
			auto_reload:false,
			catalog_mode:"tool".into(),
			expand_guidance:false,
			dev_mode:false,
			directives:std::collections::HashMap::new(),
			active_directives:Vec::new(),
			ephemeral_directives:Vec::new(),
			flow_budget_chars:4000,
			session_inject:String::new(),
			manual_directive_turn:None,
			tool_events:VecDeque::new(),
			bg_tasks:VecDeque::new(),
			poll_worker_enabled:true,
			chain_split_enabled:false,
			chain_split_min_segments:2,
			chain_split_floor:2,
			chain_split_max_segments:6,
			split_events:VecDeque::new(),
			split_segment_map:HashMap::new(),
			split_next_event_id:0,
			last_emitted_marker_count:0,
			last_emitted_file_count:0,
		}
	}
}
