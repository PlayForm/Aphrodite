use super::{AphroditeState, SPLIT_ADAPT_MIN_EVENTS, SPLIT_EVENT_CAP};

/// One chain-split event's consequence record (Tier 1 teaching loop).
/// `produced` = segment markers stored for this chain; `retrieved` =
/// how many of those distinct hashes the agent later resolved (counted at
/// most once per hash). The adaptation rule reads the retrieval ratio
/// (retrieved/produced) over the recent window to move
/// `chain_split_min_segments` within its bounds.
#[derive(Debug, Clone)]
pub struct SplitEvent {
	/// Event id (matches `split_segment_map` values).
	pub id:usize,
	/// Turn on which the chain output was split.
	pub turn:usize,
	/// Segment markers produced for this chain.
	pub produced:usize,
	/// Distinct produced hashes resolved since (≤ produced).
	pub retrieved:usize,
}

impl AphroditeState {
	/// Record a chain-split event (Tier 1 teaching loop): a chain's output
	/// was split into `produced` segment markers whose hashes are registered
	/// for retrieval attribution. Bounded ring (cap `SPLIT_EVENT_CAP`), then
	/// re-adapts the split threshold from the retrieval ratio.
	pub fn record_chain_split(&mut self, hashes:Vec<String>) {
		if hashes.is_empty() {
			return;
		}
		self.split_next_event_id += 1;
		let id = self.split_next_event_id;
		let produced = hashes.len();
		for h in &hashes {
			self.split_segment_map.insert(h.clone(), id);
		}
		self.split_events
			.push_back(SplitEvent { id, turn:self.turn_counter, produced, retrieved:0 });
		while self.split_events.len() > SPLIT_EVENT_CAP {
			self.split_events.pop_front();
		}
		self.adapt_chain_split_threshold();
	}

	/// Attribute a successful resolve to a split event (consequence signal).
	/// A hash counts at most once - removed from `split_segment_map` after
	/// attribution, so re-retrieving the same marker never double-counts.
	/// Returns the event id when the hash belonged to a chain split.
	pub fn note_split_retrieval(&mut self, hash:&str) -> Option<usize> {
		let eid = self.split_segment_map.remove(hash)?;
		// The linear scan below is bounded by SPLIT_EVENT_CAP (16) events -
		// deliberately not restructured (fix 7): an id→event map would need
		// an extra index kept in sync on ring eviction for at most 16
		// elements, so the scan is the simpler, equivalent choice. The
		// adaptation behavior this feeds is intentionally untouched.
		if let Some(ev) = self.split_events.iter_mut().find(|e| e.id == eid) {
			ev.retrieved = ev.retrieved.saturating_add(1);
		}
		self.adapt_chain_split_threshold();
		Some(eid)
	}

	/// Consequence-driven threshold adaptation (Tier 1, invisible): with at
	/// least `SPLIT_ADAPT_MIN_EVENTS` split events recorded, compute the
	/// retrieval ratio (retrieved/produced) over the window. Agents that
	/// retrieve the segments (> 50%) keep the threshold at the floor (split
	/// eagerly); agents that ignore them (< 25%) raise it (only split long
	/// chains), capped by `chain_split_max_segments`. The threshold is
	/// internal machinery - never rendered to the LLM.
	fn adapt_chain_split_threshold(&mut self) {
		if self.split_events.len() < SPLIT_ADAPT_MIN_EVENTS {
			return;
		}
		let produced:usize = self.split_events.iter().map(|e| e.produced).sum();
		let retrieved:usize = self.split_events.iter().map(|e| e.retrieved).sum();
		if produced == 0 {
			return;
		}
		let ratio = retrieved as f64 / produced as f64;
		if ratio >= 0.5 && self.chain_split_min_segments > self.chain_split_floor {
			// Segments are used - split more eagerly.
			self.chain_split_min_segments -= 1;
		} else if ratio < 0.25 && self.chain_split_min_segments < self.chain_split_max_segments {
			// Segments are ignored - only split very long chains.
			self.chain_split_min_segments += 1;
		}
	}
}
