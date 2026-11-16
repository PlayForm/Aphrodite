//! The live compression thresholds, resolved once and re-resolved on reload.
//!
//! One value object instead of six loose locals: the proxy handler, the CCR
//! creation path and `/stats` all read the same snapshot, so they can never
//! disagree about which threshold is in force mid-request. It is a plain data
//! carrier - the precedence rules live in `Resolve`.

/// Live-resolved compression thresholds (report 07 F2/F4/T15): environment
/// variable > TOML `[compression]` value > compiled-in default, the same
/// precedence pattern `Multi::Port::Override` already uses for the listen port.
/// Computed once at startup (`build_state`) and re-computed on every hot-reload
/// (config-file watcher + `POST /reload`) so both actually change the live proxy
/// instead of only re-parsing and logging.
///
/// Every field name is the word the `[compression]` key and the
/// `Proxy::State::Struct` atomic of the same meaning carry, so the resolved
/// snapshot reads as the same vocabulary it was parsed from.
pub struct Struct {
	/// `[compression] tool_threshold_cache` - body size in bytes above which
	/// cache mode compresses. `0` would compress everything.
	pub Cache:usize,
	/// `[compression] tool_threshold_token` - body size in bytes above which
	/// token mode compresses. `0` would compress everything.
	pub Token:usize,
	/// `[compression] inline_threshold` - body size in bytes below which content
	/// is kept in the inline map instead of the CCR backend. `0` disables the
	/// inline store entirely.
	pub inline:usize,
	/// `[compression] code_multiplier` - how much cheaper a token is when it
	/// carries code, applied per content family. `0.0` makes code content size
	/// zero.
	pub CodeMultiplier:f64,
	/// `[compression] auto_expand` (pre-Rust port): resolve the just-stored
	/// content inline instead of emitting a `<<<CCR:...>>>` marker, capped at
	/// `ExpandLimit` bytes. Environment variable `APHRODITE_AUTO_EXPAND`
	/// (`1`/`true`) > TOML key `auto_expand` > default `false` - the same
	/// environment > TOML > default precedence as the thresholds.
	pub AutoExpand:bool,
	/// `[compression] auto_expand_limit` - byte cap for auto-expanded content;
	/// content above it falls back to the marker. Environment variable
	/// `APHRODITE_AUTO_EXPAND_LIMIT` > TOML key `auto_expand_limit` > the
	/// compiled-in `InitialExpand`. `0` auto-expands nothing.
	pub ExpandLimit:usize,
	/// `[compression] max_request_body_size` (pre-Rust port): proxy request-body
	/// cap in bytes - requests with bodies over this size are rejected with 413
	/// while the body is being read (never fully buffered). Environment variable
	/// `APHRODITE_MAX_REQUEST_BODY_SIZE` > TOML key `max_request_body_size` >
	/// `None` (no cap = current behavior). `Some(0)` rejects every non-empty
	/// body. Resolved at startup only; write-once like the other non-atomic
	/// config (a `/reload` echoes it but cannot change a live cap).
	pub BodyLimit:Option<usize>,
}
