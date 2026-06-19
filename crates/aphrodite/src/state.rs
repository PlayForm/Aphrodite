//! headroom-ffi internal state — mirrors plugins/aphrodite/_core/state.py
//! All session-scoped state lives here: inline store, conv index, markers, counters.

use std::collections::{HashMap, VecDeque};

/// Maximum inline store entries before LRU eviction.
const INLINE_MAX: usize = 500;

/// Session state — one per loaded dylib instance.
pub struct AphroditeState {
    /// Inline content store: {hash: content}, LRU-ordered.
    pub inline_store: VecDeque<(String, String)>,
    /// Hash alias map: {full_sha256: short_hash}
    pub hash_alias: HashMap<String, String>,
    /// Recent CCR markers for catalog: [{hash, type, size, preview, turn}]
    pub recent_markers: Vec<MarkerEntry>,
    /// Conversation index: {turn_num: (hash, summary, size)}
    pub conv_index: HashMap<usize, (String, String, usize)>,
    /// Referenced files: {filepath: last_tool_name}
    pub referenced_files: VecDeque<(String, String)>,
    /// Turn counter
    pub turn_counter: usize,
    /// Scanned message index for incremental marker scan
    pub scanned_msg_idx: usize,
    /// Git cache: {timestamp, summary}
    pub git_cache: HashMap<String, String>,
    /// File tools set
    pub file_tools: Vec<String>,
    /// Config values
    pub api_url: String,
    pub model: String,
    pub engine_threshold_pct: u64,
    pub engine_min_msgs: usize,
    pub engine_protect_first: usize,
    pub engine_protect_last: usize,
    pub context_engine_enabled: bool,
    pub tool_threshold: usize,
    pub terminal_threshold: usize,
    pub catalog_mode: String,
    pub expand_guidance: bool,
    pub dev_mode: bool,
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
            hash_alias: HashMap::new(),
            recent_markers: Vec::new(),
            conv_index: HashMap::new(),
            referenced_files: VecDeque::new(),
            turn_counter: 0,
            scanned_msg_idx: 0,
            git_cache: HashMap::new(),
            file_tools: vec![
                "read_file".into(), "write_file".into(),
                "patch".into(), "search_files".into(),
            ],
            api_url: String::new(),
            model: "gpt-4o".into(),
            engine_threshold_pct: 45,
            engine_min_msgs: 8,
            engine_protect_first: 2,
            engine_protect_last: 5,
            context_engine_enabled: true,
            tool_threshold: 4096,
            terminal_threshold: 1024,
            catalog_mode: "auto".into(),
            expand_guidance: false,
            dev_mode: false,
        }
    }
}

impl AphroditeState {
    /// Insert into inline store with LRU eviction.
    pub fn inline_store_put(&mut self, hash: String, content: String) {
        // Remove existing entry if present (will be re-added at front)
        self.inline_store.retain(|(h, _)| h != &hash);
        self.inline_store.push_front((hash, content));
        // Evict oldest
        while self.inline_store.len() > INLINE_MAX {
            self.inline_store.pop_back();
        }
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

    /// Increment turn counter.
    pub fn increment_turn(&mut self) -> usize {
        self.turn_counter += 1;
        self.turn_counter
    }

    /// Reset scanned message index.
    pub fn reset_scanned(&mut self) {
        self.scanned_msg_idx = 0;
    }

    /// Reset turn counter.
    pub fn reset_turns(&mut self) {
        self.turn_counter = 0;
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
