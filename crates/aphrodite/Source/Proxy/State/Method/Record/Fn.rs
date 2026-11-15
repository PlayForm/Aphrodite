//! `AppState`'s recorders: latency histogram, error ring, per-type counts,
//! request history.
//!
//! Every one of them is best-effort: a poisoned mutex drops the sample instead
//! of propagating, because observability must never fail a request that
//! otherwise succeeded.

use std::sync::atomic::Ordering;

use crate::Proxy::State::Struct::Struct as AppState;

impl AppState {
	pub(crate) fn RecordLatency(&self, d:std::time::Duration) {
		let Us = d.as_micros() as u64;
		let Bucket = if Us < 1_000 {
			0
		} else if Us < 10_000 {
			1
		} else if Us < 100_000 {
			2
		} else if Us < 1_000_000 {
			3
		} else {
			4
		};
		self.LatencyBuckets[Bucket].fetch_add(1, Ordering::Relaxed);
		self.TotalLatencyMicros.fetch_add(Us, Ordering::Relaxed);
	}

	pub(crate) fn RecordError(&self, Message:String) {
		if let Ok(mut v) = self.LastErrors.lock() {
			v.push_back(Message);
			if v.len() > 100 {
				v.pop_front();
			}
		}
	}

	pub(crate) fn RecordCompression(&self, Ct:&str) {
		if let Ok(mut m) = self.CompressionsByType.lock() {
			*m.entry(Ct.to_string()).or_insert(0) += 1;
		}
	}

	pub(crate) fn RecordRequest(&self, ID:&str, Method:&str, path:&str, Status:u16, Compressed:bool, ElapsedMs:u128) {
		if let Ok(mut Hist) = self.RequestHistory.lock() {
			Hist.push_back(serde_json::json!({
				"id": ID,
				"method": Method,
				"path": path,
				"status": Status,
				"compressed": Compressed,
				"elapsed_ms": ElapsedMs,
			}));
			if Hist.len() > 50 {
				Hist.pop_front();
			}
		}
	}
}
