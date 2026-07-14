//! Aphrodite internal state - mirrors plugins/aphrodite/_core/state.py
//! All session-scoped state lives here: inline store, conv index, markers,
//! counters.

use std::collections::{HashMap, VecDeque};

/// Maximum inline store entries before LRU eviction.
const INLINE_MAX: usize = 500;

/// Default byte budget for the inline store (report 05 F11): entry-count
/// alone (`INLINE_MAX`) doesn't bound memory - `aphrodite_prefetch` admits
/// files up to 10MB each and the ABI admits blobs up to 16MB, so 500 entries
/// at the large end is a multi-GB worst case with zero byte accounting.
/// 256MB is a conservative default for a single agent session's compression
/// cache; exposed via `AphroditeState::inline_store_byte_budget` so a config
/// layer can override it.
pub const DEFAULT_INLINE_BYTE_BUDGET: usize = 256 * 1024 * 1024;

/// Session state - one per loaded dylib instance.
pub struct AphroditeState {
	/// Inline content store: {hash: content}, LRU-ordered.
	pub inline_store: VecDeque<(String, String)>,
	/// Running total of `content.len()` across every entry in `inline_store`,
	/// maintained incrementally by `inline_store_put` so eviction doesn't
	/// need an O(n) rescan on every insert (report 05 F11).
	inline_store_bytes: usize,
	/// Byte budget for `inline_store`; entries are evicted from the back
	/// (oldest/least-recently-used) until the running total is at or under
	/// this, in addition to the existing `INLINE_MAX` entry-count cap.
	/// Defaults to [`DEFAULT_INLINE_BYTE_BUDGET`]; see
	/// `inline_store_byte_budget`/`set_inline_store_byte_budget`.
	inline_store_byte_budget: usize,
	/// Recent CCR markers for catalog: [{hash, type, size, preview, turn}]
	pub recent_markers: Vec<MarkerEntry>,
	/// Conversation index: {turn_num: (hash, summary, size)} - the last
	/// marker archived per turn by `session::archive_turn`, called from
	/// `hooks::post_llm_call` (report 06 F11/T13: previously `archive_turn`
	/// was never called from any hook, so this stayed empty forever and
	/// `aphrodite_diff` always returned zero turns).
	pub conv_index: HashMap<usize, (String, String, usize)>,
	/// Referenced files: {filepath: last_tool_name}
	pub referenced_files: VecDeque<(String, String)>,
	/// Turn counter.
	pub turn_counter: usize,
	/// Scanned message index for incremental marker scan.
	pub scanned_msg_idx: usize,
	/// File tools set.
	pub file_tools: Vec<String>,
	// ── Config values (mirrored from aphrodite.toml) ──
	pub api_url: String,
	pub model: String,
	pub engine_threshold_pct: u64,
	// RESERVED: write-only today (loaded from aphrodite.toml, never read back
	// by the proxy) - candidate consumers for the context-engine work
	// (13-P2), not deleted since that work may land on them directly
	// (01-F9, user decision: keep-reserved over delete).
	pub engine_min_msgs: usize,
	pub engine_protect_first: usize,
	pub engine_protect_last: usize,
	pub context_engine_enabled: bool,
	pub tool_threshold: usize,
	pub terminal_threshold: usize,
	// RESERVED: same as engine_min_msgs above (01-F9).
	pub catalog_mode: String,
	pub expand_guidance: bool,
	pub dev_mode: bool,
	// ── Conversational Directives ──
	/// All loaded directives (name → content).
	pub directives: std::collections::HashMap<String, crate::directives::Directive>,
	/// Currently active directive names (the ones injected into context).
	pub active_directives: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarkerEntry {
	pub hash: String,
	pub ccr_type: String,
	pub size: usize,
	pub preview: String,
	pub turn: usize,
	pub center: Option<String>,
	pub meta: Option<HashMap<String, String>>,
}

impl Default for AphroditeState {
	fn default() -> Self {
		Self {
			inline_store: VecDeque::with_capacity(INLINE_MAX),
			inline_store_bytes: 0,
			inline_store_byte_budget: DEFAULT_INLINE_BYTE_BUDGET,
			recent_markers: Vec::new(),
			conv_index: HashMap::new(),
			referenced_files: VecDeque::new(),
			turn_counter: 0,
			scanned_msg_idx: 0,
			file_tools: vec!["read_file".into(), "write_file".into(), "patch".into(), "search_files".into()],
			api_url: String::new(),
			model: "gpt-4o".into(),
			engine_threshold_pct: 45,
			engine_min_msgs: 8,
			engine_protect_first: 2,
			engine_protect_last: 5,
			context_engine_enabled: true,
			tool_threshold: 4096,
			terminal_threshold: 1024,
			catalog_mode: "tool".into(),
			expand_guidance: false,
			dev_mode: false,
			directives: std::collections::HashMap::new(),
			active_directives: Vec::new(),
		}
	}
}

impl AphroditeState {
	/// Current byte budget for the inline store (report 05 F11).
	pub fn inline_store_byte_budget(&self) -> usize {
		self.inline_store_byte_budget
	}

	/// Override the inline store's byte budget (e.g. from config); evicts
	/// immediately if the new budget is lower than the current usage.
	pub fn set_inline_store_byte_budget(&mut self, budget: usize) {
		self.inline_store_byte_budget = budget;
		self.evict_over_budget();
	}

	/// Current total bytes held across every entry in the inline store.
	pub fn inline_store_bytes(&self) -> usize {
		self.inline_store_bytes
	}

	/// Evict from the back (oldest/least-recently-used) until both the
	/// entry-count cap (`INLINE_MAX`) and the byte budget
	/// (`inline_store_byte_budget`) are satisfied.
	fn evict_over_budget(&mut self) {
		while self.inline_store.len() > INLINE_MAX
			|| (self.inline_store_bytes > self.inline_store_byte_budget && !self.inline_store.is_empty())
		{
			if let Some((_, c)) = self.inline_store.pop_back() {
				self.inline_store_bytes = self.inline_store_bytes.saturating_sub(c.len());
			} else {
				break;
			}
		}
	}

	/// Insert into inline store with LRU + byte-budget eviction (report 05
	/// F11: previously bounded by entry count only - `aphrodite_prefetch`
	/// admits files up to 10MB each and the ABI admits blobs up to 16MB, so
	/// 500 entries at the large end is a multi-GB worst case).
	pub fn inline_store_put(&mut self, hash: String, content: String) {
		// Remove existing entry if present (will be re-added at front),
		// keeping the running byte total in sync.
		if let Some(pos) = self.inline_store.iter().position(|(h, _)| h == &hash) {
			if let Some((_, old)) = self.inline_store.remove(pos) {
				self.inline_store_bytes = self.inline_store_bytes.saturating_sub(old.len());
			}
		}
		self.inline_store_bytes += content.len();
		self.inline_store.push_front((hash, content));
		self.evict_over_budget();
	}

	/// Retrieve from inline store with LRU promotion.
	pub fn inline_store_get(&mut self, hash: &str) -> Option<String> {
		if let Some(pos) = self.inline_store.iter().position(|(h, _)| h == hash) {
			let (h, c) = self.inline_store.remove(pos).unwrap();
			self.inline_store.push_front((h, c.clone()));
			Some(c)
		} else {
			None
		}
	}

	/// Record a compression marker.
	pub fn record_marker(&mut self, entry: MarkerEntry) {
		self.recent_markers.push(entry);
		// Keep last 200 markers
		while self.recent_markers.len() > 200 {
			self.recent_markers.remove(0);
		}
	}

	/// Record a referenced file.
	pub fn record_file(&mut self, path: String, tool: String) {
		self.referenced_files.retain(|(p, _)| p != &path);
		self.referenced_files.push_front((path, tool));
		while self.referenced_files.len() > 100 {
			self.referenced_files.pop_back();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_inline_store_put_get() {
		let mut s = AphroditeState::default();
		s.inline_store_put("abc".into(), "hello world".into());
		assert_eq!(s.inline_store_get("abc"), Some("hello world".into()));
	}

	#[test]
	fn test_inline_store_missing() {
		let mut s = AphroditeState::default();
		assert_eq!(s.inline_store_get("nope"), None);
	}

	#[test]
	fn test_inline_store_lru_promotion() {
		let mut s = AphroditeState::default();
		s.inline_store_put("a".into(), "first".into());
		s.inline_store_put("b".into(), "second".into());
		// Get "a" promotes it to front
		let _ = s.inline_store_get("a");
		// "a" should now be at front
		let front = s.inline_store.pop_front();
		assert_eq!(front, Some(("a".into(), "first".into())));
	}

	#[test]
	fn test_inline_store_eviction() {
		let mut s = AphroditeState::default();
		// Fill beyond INLINE_MAX (500)
		for i in 0..505 {
			s.inline_store_put(format!("hash{}", i), format!("content{}", i));
		}
		assert!(s.inline_store.len() <= 500);
		// Oldest should be evicted
		assert_eq!(s.inline_store_get("hash0"), None);
		// Newest should remain
		assert_eq!(s.inline_store_get("hash504"), Some("content504".into()));
	}

	// ── T11 (F11): byte-budget eviction ───────────────────────────
	#[test]
	fn test_inline_store_byte_budget_evicts_oldest_first() {
		let mut s = AphroditeState::default();
		s.set_inline_store_byte_budget(10 * 1024 * 1024); // 10MB budget
		// 100 x 5MB entries (500MB total) - far beyond both the byte budget
		// and, at this size, would also never be reached by the 500-entry
		// cap, so this specifically exercises the byte accounting rather
		// than the pre-existing entry-count cap.
		let five_mb = "x".repeat(5 * 1024 * 1024);
		for i in 0..100 {
			s.inline_store_put(format!("hash{i}"), five_mb.clone());
		}
		assert!(
			s.inline_store_bytes() <= 10 * 1024 * 1024,
			"stored bytes ({}) must stay within the 10MB budget",
			s.inline_store_bytes()
		);
		// Oldest entries must be the ones evicted.
		assert_eq!(s.inline_store_get("hash0"), None);
		// The newest entry must survive.
		assert_eq!(s.inline_store_get("hash99"), Some(five_mb));
	}

	#[test]
	fn test_inline_store_default_byte_budget_is_256mb() {
		let s = AphroditeState::default();
		assert_eq!(s.inline_store_byte_budget(), 256 * 1024 * 1024);
	}

	#[test]
	fn test_inline_store_lowering_budget_evicts_immediately() {
		let mut s = AphroditeState::default();
		s.inline_store_put("a".into(), "x".repeat(1000));
		s.inline_store_put("b".into(), "x".repeat(1000));
		assert_eq!(s.inline_store_bytes(), 2000);
		// Below current usage (2000B) but large enough for the single
		// most-recent entry ("b", 1000B) to survive on its own.
		s.set_inline_store_byte_budget(1500);
		assert!(s.inline_store_bytes() <= 1500);
		// The most recently inserted entry ("b") must be the one kept.
		assert_eq!(s.inline_store_get("b"), Some("x".repeat(1000)));
		assert_eq!(
			s.inline_store_get("a"),
			None,
			"oldest entry must have been evicted to fit the new budget"
		);
	}

	#[test]
	fn test_record_marker_eviction() {
		let mut s = AphroditeState::default();
		for i in 0..250 {
			s.record_marker(MarkerEntry {
				hash: format!("h{}", i),
				ccr_type: "text".into(),
				size: 100,
				preview: "[text]".into(),
				turn: i,
				center: None,
				meta: None,
			});
		}
		assert!(s.recent_markers.len() <= 200);
		assert_eq!(s.recent_markers[0].hash, "h50"); // First 50 evicted
	}

	#[test]
	fn test_record_file_dedup() {
		let mut s = AphroditeState::default();
		s.record_file("/tmp/a".into(), "read".into());
		s.record_file("/tmp/a".into(), "write".into());
		assert_eq!(s.referenced_files.len(), 1);
		assert_eq!(s.referenced_files[0].1, "write"); // Updated tool
	}

	#[test]
	fn test_default_values() {
		let s = AphroditeState::default();
		assert_eq!(s.turn_counter, 0);
		assert_eq!(s.tool_threshold, 4096);
		assert_eq!(s.terminal_threshold, 1024);
		assert!(s.context_engine_enabled);
	}

	#[test]
	fn test_inline_store_put_overwrite() {
		let mut s = AphroditeState::default();
		s.inline_store_put("hash".into(), "v1".into());
		s.inline_store_put("hash".into(), "v2".into());
		assert_eq!(s.inline_store.len(), 1);
		assert_eq!(s.inline_store_get("hash"), Some("v2".into()));
	}
}
