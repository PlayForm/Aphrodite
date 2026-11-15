//! The caps and budgets every bounded structure in the state enforces.
//!
//! One file because they are one decision: how much a single agent session is
//! allowed to hold. A cap that lives next to the ring it bounds gets tuned in
//! isolation and drifts from the others; keeping them together makes the
//! memory profile of a session readable at a glance.
//!
//! None of these is observable to the LLM - they change what survives, never
//! what is rendered.

/// Maximum inline store entries before LRU eviction.
pub(crate) const InlineMaximum:usize = 500;

/// Default byte budget for the inline store (report 05 F11): entry-count
/// alone (`InlineMaximum`) doesn't bound memory - `aphrodite_prefetch` admits
/// files up to 10MB each and the ABI admits blobs up to 16MB, so 500 entries
/// at the large end is a multi-GB worst case with zero byte accounting.
/// 256MB is a conservative default for a single agent session's compression
/// cache; exposed via `AphroditeState::inline_store_byte_budget` so a config
/// layer can override it.
pub const DefaultInlineByteBudget:usize = 256 * 1024 * 1024;

/// Cap on the chain-split consequence ledger (events kept for adaptation).
pub(crate) const SplitEventCap:usize = 16;

/// Cap on the per-tool-call telemetry ring (P2); oldest evicted from the
/// front.
pub(crate) const ToolEventCap:usize = 200;

/// Cap on the recent-marker catalog; oldest evicted from the front.
pub(crate) const RecentMarkersCap:usize = 200;

/// Cap on the referenced-files list; least-recently-referenced evicted.
pub(crate) const ReferencedFilesCap:usize = 100;

/// Minimum split events recorded before the threshold adapts - avoids
/// adapting on noise from a single split.
pub(crate) const SplitAdaptMinimumEvents:usize = 4;
