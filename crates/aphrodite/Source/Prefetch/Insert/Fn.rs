//! The classify-and-store pass: the only part of prefetch that needs the
//! lock.
//!
//! It takes already-read outcomes and does three things - build the
//! per-path JSON report, put loaded content in the CCR store, record the
//! marker and the touched file. No I/O happens here, which is why it can hold
//! the state lock safely. The response also surfaces the inline store's
//! current usage and budget: a prefetched batch can evict older entries
//! (including ones from the same batch), and a caller that cannot see the
//! budget discovers that only when a later retrieve unexpectedly misses.

use crate::Prefetch::Arrival::Struct::LiveArrival;
use crate::Prefetch::Outcome::Enum::ReadOutcome;
use crate::Prefetch::Read::Const::MaximumPrefetchSize;
use crate::State::Struct::AphroditeState;
use crate::State::Marker::Struct::MarkerEntry;

/// Classify and store already-read file contents into `State`. Pure state
/// mutation + JSON assembly - no I/O, so this is the only part that needs
/// the lock.
pub fn InsertOutcomes(State:&mut AphroditeState, Outcomes:Vec<(String, ReadOutcome)>) -> serde_json::Value {
	let Total = Outcomes.len();
	let mut Results = Vec::with_capacity(Total);
	let mut Loaded = 0u32;
	let mut SkippedSize = 0u32;
	let mut Refused = 0u32;
	let mut Missing = 0u32;

	for (PathStr, Outcome) in Outcomes {
		match Outcome {
			ReadOutcome::Missing | ReadOutcome::Error => {
				Missing += 1;
				Results.push(serde_json::json!({"path": PathStr, "status": "missing"}));
			},
			ReadOutcome::Refused { Reason } => {
				Refused += 1;
				Results.push(serde_json::json!({
					"path": PathStr,
					"status": "refused",
					"reason": Reason,
				}));
			},
			ReadOutcome::SkippedSize { Size } => {
				SkippedSize += 1;
				Results.push(serde_json::json!({
					"path": PathStr,
					"status": "skipped",
					"reason": format!(
						"exceeds per-file prefetch limit ({} bytes > {} byte max)",
						Size, MaximumPrefetchSize
					),
					"size": Size,
				}));
			},
			ReadOutcome::Loaded { Content, Size } => {
				let Ct = headroom_core::transforms::content_detector::detect_content_type(&Content).content_type;
				let hash = headroom_core::ccr::compute_key(Content.as_bytes());
				let TypeStr = Ct.as_str();
				let Preview = crate::Preview::Builder::Entry::Fn(TypeStr, &Content);

				State.InlineStorePut(hash.clone(), Content);
				State.RecordMarker(MarkerEntry {
					hash:hash.clone(),
					CCRType:TypeStr.to_string(),
					Size:Size as usize,
					Preview:Preview.clone(),
					Turn:State.TurnCounter,
					Center:None,
					meta:Some({
						let mut m = std::collections::HashMap::new();
						m.insert("path".to_string(), PathStr.clone());
						// Live-container mode (pre-Rust `_hooks/live.py`):
						// the schedule entry carries container semantics -
						// arrival timestamp, byte progress, ETA - so the
						// prefetch_status surface can expose them. Off by
						// default: meta keeps only `path` (byte-identical
						// current behavior).
						if State.LiveContainer {
							let Arrival = LiveArrival::Arrived(Size);
							m.insert("live".to_string(), "1".to_string());
							m.insert("live_arrived_at".to_string(), Arrival.ArrivedAtUnix.to_string());
							m.insert("live_bytes_arrived".to_string(), Arrival.BytesArrived.to_string());
							m.insert("live_eta_s".to_string(), Arrival.EtaS.to_string());
						}
						m
					}),
				});

				Loaded += 1;
				State.RecordFile(PathStr.clone(), "prefetch".to_string());

				Results.push(serde_json::json!({
					"path": PathStr,
					"status": "loaded",
					// Full 40-char hash - a truncated one is unresolvable via
					// exact-match retrieval (report 05 F3).
					"hash": &hash,
					"type": TypeStr,
					"size": Size,
					"preview": Preview,
				}));
			},
		}
	}

	serde_json::json!({
		"total": Total,
		"loaded": Loaded,
		"skipped_size": SkippedSize,
		"refused": Refused,
		"missing": Missing,
		"results": Results,
		// Report 05 F11: prefetched content joins the same byte-budgeted
		// inline store as everything else, and can silently evict older
		// entries (including ones this very batch just loaded, if the batch
		// itself exceeds the budget) - surface the current usage/budget so a
		// caller can tell whether that happened instead of discovering it
		// only when a later retrieve unexpectedly misses.
		"inline_store_bytes": State.InlineStoreBytes(),
		"inline_store_byte_budget": State.InlineStoreByteBudget(),
	})
}
