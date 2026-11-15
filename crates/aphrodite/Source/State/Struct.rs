//! The session state struct.
//!
//! This file holds the fields and their invariants - no `impl`. Every method
//! on `AphroditeState` lives in the directory that owns the data it touches
//! (`Store/`, `Marker/`, `Event/`, `File/`, `ChainSplit/`), so a change to the
//! inline store can never reach into the marker catalog by accident.
//!
//! Field comments are load-bearing: several of them record why a structure was
//! chosen (`HashMap` + `LruCache` over a `VecDeque`, `VecDeque` rings with
//! front eviction) and which bug a later change would reintroduce by
//! "simplifying" it.

use std::collections::{HashMap, VecDeque};

use crate::State::ChainSplit::Struct::SplitEvent;
use crate::State::Directive::Struct::ActiveDirective;
use crate::State::Event::Struct::ToolEvent;
use crate::State::Marker::Struct::MarkerEntry;
use crate::State::Template::Struct::TemplatesConfiguration;

/// Session state - one per loaded dylib instance.
///
/// `Clone` enables the hermes bridge's per-session registry: each new
/// session's state is a clone of a config-seeded template, so subagent
/// spawns inherit the same thresholds/directives without re-reading config.
/// Cloning is a deep copy of the (empty at seed time) CCR stores.
#[derive(Clone)]
pub struct AphroditeState {
	/// Inline content store: {hash: content}. The `HashMap` gives O(1)
	/// get/put/contains (was an O(n) `VecDeque` linear scan per op - bug
	/// 18-P14); `InlineOrder` preserves LRU recency so eviction can drop
	/// the least-recently-used entry in O(1).
	pub InlineStore:HashMap<String, String>,
	/// LRU recency order of `InlineStore` keys (unit values). Capacity is
	/// `INLINE_MAX + 1` so the cache never self-evicts before the explicit
	/// `EvictOverBudget` loop in `InlineStorePut` (fix 1: previously a
	/// `VecDeque` whose promotion/upsert needed an O(n) `retain` scan).
	pub InlineOrder:lru::LruCache<String, ()>,
	/// Running total of `content.len()` across every entry in `InlineStore`,
	/// maintained incrementally by `InlineStorePut` so eviction doesn't
	/// need an O(n) rescan on every insert (report 05 F11).
	pub(super) InlineStoreBytes:usize,
	/// Byte budget for `InlineStore`; entries are evicted from the back
	/// (oldest/least-recently-used) until the running total is at or under
	/// this, in addition to the existing `InlineMaximum` entry-count cap.
	/// Defaults to [`super::DEFAULT_INLINE_BYTE_BUDGET`]; see
	/// `InlineStoreByteBudget`/`SetInlineStoreByteBudget`.
	pub(super) InlineStoreByteBudget:usize,
	/// Recent CCR markers for catalog: [{hash, type, size, preview, turn}].
	/// `VecDeque` (fix 6): `RecordMarker` pushes back and pops the front
	/// while over `RecentMarkersCap`, so the oldest marker always sits at
	/// index 0.
	pub RecentMarkers:VecDeque<MarkerEntry>,
	/// Conversation index: {turn_num: (hash, summary, size)} - the last
	/// marker archived per turn by `Session::archive_turn`, called from
	/// `Hook::LLM::Post::Fn::Fn` (report 06 F11/T13: previously `ArchiveTurn`
	/// was never called from any hook, so this stayed empty forever and
	/// `aphrodite_diff` always returned zero turns).
	pub ConvIndex:HashMap<usize, (String, String, usize)>,
	/// Referenced files: {filepath: last_tool_name}
	pub ReferencedFiles:VecDeque<(String, String)>,
	/// Turn counter.
	pub TurnCounter:usize,
	/// Scanned message index for incremental marker scan.
	pub ScannedMessageIndex:usize,
	/// File tools set.
	pub FileTools:Vec<String>,
	// ── Config values (mirrored from aphrodite.toml) ──
	pub APIURL:String,
	pub Model:String,
	pub EngineThresholdPct:u64,
	// RESERVED: write-only today (loaded from aphrodite.toml, never read back
	// by the proxy) - candidate consumers for the context-engine work
	// (13-P2), not deleted since that work may land on them directly
	// (01-F9, user decision: keep-reserved over delete).
	pub EngineMinimumMsgs:usize,
	pub EngineProtectFirst:usize,
	pub EngineProtectLast:usize,
	pub ContextEngineEnabled:bool,
	pub ToolThreshold:usize,
	pub TerminalThreshold:usize,
	/// Parse-failure self-diagnosis: set when the loaded aphrodite.toml was
	/// found but failed to parse (defaults in effect). Surfaced verbatim in
	/// `AphroditeStats` so a broken config is never indistinguishable from
	/// "config not set" - the tracing warn is a no-op in the Hermes dylib.
	pub ConfigurationError:Option<String>,
	/// Opt-in config auto-reload: watch aphrodite.toml and re-apply config
	/// fields on change (default off). Only config fields are mutated -
	/// session CCR state, directive selection, and telemetry are preserved.
	pub AutoReload:bool,
	/// DEV-ONLY debug logger level (0 = off, 1 = on, 2 = verbose), read from
	/// `APHRODITE_DEBUG` at config load (temporary development
	/// instrumentation - see `crate::Debug`; never a release path).
	pub DebugLevel:u8,
	// RESERVED: same as engine_min_msgs above (01-F9).
	pub CatalogMode:String,
	pub ExpandGuidance:bool,
	pub DevMode:bool,
	// ── Conversational Directives ──
	/// All loaded directives (name → content).
	pub Directives:std::collections::HashMap<String, crate::Directive::Model::Struct>,
	/// Currently active directive names (the ones injected into context).
	pub ActiveDirectives:Vec<String>,
	/// Ephemeral (one-shot / TTL) directives - inline nudges that render once
	/// (or for a bounded number of turns) then self-purge (P3/T9). Distinct
	/// from `ActiveDirectives` (permanent-until-removed named entries).
	pub EphemeralDirectives:Vec<ActiveDirective>,
	// ── Flow context assembler (P1) ──
	/// Hard cap for ALL per-turn injected context assembled by
	/// `Flow::build_turn_context` (default 4000 chars, `[flow] budget_chars`).
	pub FlowBudgetChars:usize,
	/// First-turn session instruction loaded from `[prompts] session_inject`
	/// in aphrodite.toml - rendered once via `BuildFirstTurnInjection`,
	/// then dropped (turn_counter > 0). Empty = no injection (default).
	pub SessionInject:String,
	/// `[templates]` section (pre-Rust port) - preview families, marker block,
	/// prompt templates, reverse key map. None = hardcoded builders/defaults.
	pub Templates:Option<TemplatesConfiguration>,
	/// `[compression] auto_expand` (pre-Rust port): when true, compressed
	/// tool outputs are resolved inline instead of emitting markers (capped
	/// by `AutoExpandLimit` bytes). Default false = markers appear.
	pub AutoExpand:bool,
	/// `[compression] auto_expand_limit` - byte cap for auto-expanded content.
	pub AutoExpandLimit:usize,
	/// `APHRODITE_LIVE_CONTAINER` (pre-Rust port): streaming live-container
	/// mode for tool outputs (chunked arrival + instant markers + progress).
	/// Default false = current behavior.
	pub LiveContainer:bool,
	/// `APHRODITE_RECURSIVE_DEPTH` (pre-Rust port): max marker recursion depth.
	/// Default 5 = current hardcoded RECURSIVE_DEPTH.
	pub RecursiveDepth:usize,
	/// `APHRODITE_MAX_REQUEST_BODY_SIZE` (pre-Rust port): proxy request-body
	/// cap in bytes. None = no cap (current behavior).
	pub MaximumRequestBodySize:Option<usize>,
	/// `[prompts] retrieve_guidance` (minimal|standard|verbose) - how much CCR
	/// retrieval explanation `Flow::build_turn_context` injects per turn.
	/// Default "minimal" = current behavior (no extra guidance).
	pub RetrieveGuidance:String,
	/// `[prompts] catalog_intent_hints` - show `intent=read` alongside the
	/// recent CCR hashes in the recall catalog output (default off = current).
	pub CatalogIntentHints:bool,
	/// Turn number of the most recent MANUAL `AphroditeDirective` mutation
	/// (swap/add/remove/reset). Latches phase-aware auto-swaps out (P6); set by
	/// `Directive::Action::Fn` on any successful mutation.
	pub ManualDirectiveTurn:Option<usize>,
	// ── Turn-telemetry spine (P2) ──
	/// Bounded ring of per-tool-call events (cap 200, evict front). Feeds phase
	/// detection, error-loop breaking, delta previews, checkpoints (P6-P11).
	pub ToolEvents:VecDeque<ToolEvent>,
	// ── Poll-worker auto-backgrounding ──
	/// Background tasks created by the poll-worker auto-backgrounding
	/// heuristic (cap 4, evict oldest completed/stale on overflow).
	pub BgTasks:VecDeque<crate::PollWorker::Task::Struct::BgTask>,
	/// Master on/off for poll-worker auto-backgrounding. When false,
	/// no tool output is auto-backgrounded (existing bg_tasks still
	/// receive lifecycle nudges and expiry). Default true. Env:
	/// `APHRODITE_POLL_WORKER`, TOML: `[compression] poll_worker`.
	pub PollWorkerEnabled:bool,
	/// Fine-grained chain splitting: rewrite chained shell commands
	/// (`a && b && c`) with segment markers and split the output into
	/// per-segment CCR entries, so the agent sees N compact previews
	/// instead of one giant blob. Struct default false (fix 2), matching
	/// the shipped config default; `ApplyCompression` resolves the TOML/env
	/// override (opt-in per session via `APHRODITE_CHAIN_SPLIT=1` or TOML
	/// `[compression] chain_split`). Env: `APHRODITE_CHAIN_SPLIT`, TOML:
	/// `[compression] chain_split`.
	pub ChainSplitEnabled:bool,
	// ── Tier 1 teaching loop: adaptive split threshold ──
	/// Current minimum segment count for chain splitting. Only chains with
	/// at least this many segments are rewritten. Starts at the configured
	/// floor (default 2, `SplitChain` already requires ≥2) and adapts
	/// within `[chain_split_floor, chain_split_max_segments]` based on
	/// whether the agent actually retrieves the produced segment markers
	/// (consequence-driven learning). The threshold is machinery - never
	/// observable to the LLM (no directive text, no summary changes).
	pub ChainSplitMinimumSegments:usize,
	/// Lower bound of the adaptive threshold: the configured initial value.
	pub ChainSplitFloor:usize,
	/// Upper bound of the adaptive threshold. Env:
	/// `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS`, TOML:
	/// `[compression] chain_split_max_segments`. Default 6.
	pub ChainSplitMaximumSegments:usize,
	/// Consequence ledger: one entry per chain-split event recording how
	/// many segment markers were produced and how many distinct ones were
	/// later retrieved. Bounded ring (cap `SplitEventCap`); oldest
	/// evicted. `SplitSegmentMap` maps each produced segment hash to its
	/// event id so a resolve can be attributed to the right event.
	pub SplitEvents:VecDeque<SplitEvent>,
	/// Segment hash → split-event id, for retrieval attribution. A hash
	/// resolves at most once (removed after counting), so re-retrieving the
	/// same marker never double-counts.
	pub SplitSegmentMap:HashMap<String, usize>,
	/// Monotonic event id counter for `SplitEvents`.
	pub SplitNextEventID:usize,
	// ── Delta catalog (04-F1) ──
	/// Number of markers the last time catalog_summary rendered, so we emit a
	/// delta line only when new markers arrived this turn. Zero-initialized;
	/// reset on session start. Stops the prompt-cache-poisoning repetition of
	/// the same 5 previews every turn.
	pub LastEmittedMarkerCount:usize,
	/// Number of referenced files the last time catalog_summary rendered,
	/// for delta-only file listing (04-F4: stops re-listing same 5 files).
	pub LastEmittedFileCount:usize,
}
