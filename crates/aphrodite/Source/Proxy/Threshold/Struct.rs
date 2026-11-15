//! The live compression thresholds, resolved once and re-resolved on reload.
//!
//! One value object instead of six loose locals: the proxy handler, the CCR
//! creation path and `/stats` all read the same snapshot, so they can never
//! disagree about which threshold is in force mid-request. It is a plain data
//! carrier - the precedence rules live in `Resolve`.

/// Live-resolved compression thresholds (report 07 F2/F4/T15): env var >
/// TOML `[compression]` value > compiled-in default, the same precedence
/// pattern `ApplyPortOverride` already uses for the listen port. Computed
/// once at startup (`build_state`) and re-computed on every hot-reload
/// (config-file watcher + `POST /reload`) so both actually change the live
/// proxy instead of only re-parsing and logging.
pub struct Struct {
	pub Cache:usize,
	pub Token:usize,
	pub inline:usize,
	pub CodeMultiplier:f64,
	/// `[compression] auto_expand` (pre-Rust port): resolve the just-stored
	/// content inline instead of emitting a `<<<CCR:...>>>` marker, capped
	/// at `AutoExpandLimit` bytes. Env `APHRODITE_AUTO_EXPAND`
	/// (`1`/`true`) > TOML `AutoExpand` > default false - the same
	/// env > TOML > default precedence as the thresholds.
	pub AutoExpand:bool,
	/// `[compression] auto_expand_limit` - byte cap for auto-expanded
	/// content. Env `APHRODITE_AUTO_EXPAND_LIMIT` > TOML
	/// `AutoExpandLimit` > `AutoExpandLimitDefault`.
	pub AutoExpandLimit:usize,
	/// `APHRODITE_MAX_REQUEST_BODY_SIZE` (pre-Rust port): proxy request-body
	/// cap in bytes - requests with bodies over this size are rejected with
	/// 413 while the body is being read (never fully buffered). Env >
	/// TOML `[compression] max_request_body_size` > None (no cap = current
	/// behavior). Resolved at startup only; write-once like the other
	/// non-atomic config (a `/reload` echoes it but cannot change a live cap).
	pub MaximumRequestBodySize:Option<usize>,
}
