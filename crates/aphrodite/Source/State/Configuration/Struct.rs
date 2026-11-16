//! The resolved configuration the session mirrors.
//!
//! Split out of `AphroditeState` so the config readers take
//! `&Configuration::Struct` instead of the whole session state: every one of
//! these values is written once by `ConfigurationLoader::Configuration::Apply`
//! and read many times by the hooks, the proxy and the flow assembler, and none
//! of them touches a store, a ring or a counter.
//!
//! These names keep their subsystem qualifier (`ToolThreshold` beside
//! `TerminalThreshold` beside `EngineThresholdPct`) because the path
//! `State::Configuration` says *that* these are configuration, not *which*
//! subsystem each knob belongs to. Stripping the qualifier would collapse three
//! different thresholds onto one word inside this struct - the collision rule
//! in `.hermes/notes/RUST-NAMING-DOCTRINE.md` §4 keeps the distinction. The one
//! qualifier the path does carry is `Configuration`, so `Configuration::Error`
//! drops to `Error`.

use crate::State::Template::Struct::Struct as Templates;

/// Every value the session mirrors out of `aphrodite.toml` (plus the CLI/env
/// overrides that feed the same keys).
#[derive(Clone)]
pub struct Struct {
	pub URL:String,
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
	pub Error:Option<String>,
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
	/// Hard cap for ALL per-turn injected context assembled by
	/// `Flow::build_turn_context` (default 4000 chars, `[flow] budget_chars`).
	pub FlowBudgetChars:usize,
	/// First-turn session instruction loaded from `[prompts] session_inject`
	/// in aphrodite.toml - rendered once via `BuildFirstTurnInjection`,
	/// then dropped (turn_counter > 0). Empty = no injection (default).
	pub SessionInject:String,
	/// `[templates]` section (pre-Rust port) - preview families, marker block,
	/// prompt templates, reverse key map. None = hardcoded builders/defaults.
	pub Templates:Option<Templates>,
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
}
