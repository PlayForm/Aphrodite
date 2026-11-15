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

		let Bucket = match () {
			_ if Us < 1_000 => 0,
			_ if Us < 10_000 => 1,
			_ if Us < 100_000 => 2,
			_ if Us < 1_000_000 => 3,
			_ => 4,
		};

		self.LatencyBuckets[Bucket].fetch_add(1, Ordering::Relaxed);
		self.TotalLatencyMicros.fetch_add(Us, Ordering::Relaxed);
	}

	pub(crate) fn RecordError(&self, Message:String) {
		match self.LastErrors.lock() {
			Ok(mut v) => {
				v.push_back(Message);

				match v.len() > 100 {
					true => {
						v.pop_front();
					},
					false => {},
				}
			},
			Err(_) => {},
		}
	}

	pub(crate) fn RecordCompression(&self, Ct:&str) {
		match self.CompressionsByType.lock() {
			Ok(mut m) => {
				*m.entry(Ct.to_string()).or_insert(0) += 1;
			},
			Err(_) => {},
		}
	}

	pub(crate) fn RecordRequest(&self, ID:&str, Method:&str, path:&str, Status:u16, Compressed:bool, ElapsedMs:u128) {
		match self.RequestHistory.lock() {
			Ok(mut Hist) => {
				Hist.push_back(serde_json::json!({
					"id": ID,
					"method": Method,
					"path": path,
					"status": Status,
					"compressed": Compressed,
					"elapsed_ms": ElapsedMs,
				}));

				match Hist.len() > 50 {
					true => {
						Hist.pop_front();
					},
					false => {},
				}
			},
			Err(_) => {},
		}
	}
}
