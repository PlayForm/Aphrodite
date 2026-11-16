//! `AppState`'s recorders: latency histogram, error ring, per-type counts,
//! request history.
//!
//! Every one of them is best-effort: a poisoned mutex drops the sample instead
//! of propagating, because observability must never fail a request that
//! otherwise succeeded.

use std::sync::atomic::Ordering;

use crate::Proxy::State::{Method::Record::Cap, Struct::Struct as AppState};

/// Upper bounds of the four bounded latency buckets, in microseconds.
const BucketLimits:[u64; 4] = [1_000, 10_000, 100_000, 1_000_000];

impl AppState {
	pub(crate) fn RecordLatency(&self, d:std::time::Duration) {
		let Us = d.as_micros() as u64;
		// The bucket is the position of the first limit the sample falls under;
		// past all four it is the overflow bucket. Arithmetic instead of a
		// guarded chain, and the bounds are one array a reader can compare
		// against the histogram labels.
		let Bucket = BucketLimits.iter().position(|Limit| Us < *Limit).unwrap_or(BucketLimits.len());

		self.LatencyBuckets[Bucket].fetch_add(1, Ordering::Relaxed);
		self.TotalLatencyMicros.fetch_add(Us, Ordering::Relaxed);
	}

	pub(crate) fn RecordError(&self, Message:String) {
		let _ = self.LastErrors.lock().map(|mut Errors| {
			Errors.push_back(Message);

			Cap::Fn::Fn(&mut Errors, 100);
		});
	}

	pub(crate) fn RecordCompression(&self, Ct:&str) {
		let _ = self.CompressionsByType.lock().map(|mut Counts| {
			*Counts.entry(Ct.to_owned()).or_insert(0) += 1;
		});
	}

	pub(crate) fn RecordRequest(&self, ID:&str, Method:&str, path:&str, Status:u16, Compressed:bool, ElapsedMs:u128) {
		let _ = self.RequestHistory.lock().map(|mut History| {
			History.push_back(serde_json::json!({
				"id": ID,
				"method": Method,
				"path": path,
				"status": Status,
				"compressed": Compressed,
				"elapsed_ms": ElapsedMs,
			}));

			Cap::Fn::Fn(&mut History, 50);
		});
	}
}
