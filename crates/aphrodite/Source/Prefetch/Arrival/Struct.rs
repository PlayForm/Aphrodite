//! Live-container arrival records for prefetched files.
//!
//! `LiveArrival` is a description of how much of a container has landed and
//! when - not a scheduler and not a progress bar. It exists because the
//! pre-Rust `_prefetch_registry` tracked chunked arrival per path, and the
//! status surface has to answer "is this ready yet" with a number rather than
//! a guess. The synchronous Rust read always arrives in one chunk, so
//! `arrived()` is the normal constructor; `EtaEstimate` keeps the chunked
//! model available and honest for partial records.

/// Live-container arrival record (pre-Rust `_hooks/live.py` container
/// semantics): one per prefetched entry, only surfaced when
/// `state.live_container` is on. `bytes_arrived == size` with `eta_s == 0.0`
/// means fully arrived; a partial record models chunked arrival (the
/// async-loading case the pre-Rust `_prefetch_registry` tracked per path).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveArrival {
	/// Unix-seconds timestamp of first arrival.
	pub ArrivedAtUnix:u64,
	/// Bytes of container content arrived so far.
	pub BytesArrived:u64,
	/// Total container size in bytes.
	pub Size:u64,
	/// Estimated seconds until fully arrived (0.0 = fully arrived).
	pub EtaS:f32,
}

impl LiveArrival {
	/// Fully-arrived record for content that landed synchronously (the
	/// Rust port's prefetch read is synchronous, so every loaded entry
	/// arrives in one chunk at insert time).
	///
	/// The timestamp is an argument, not a clock read: one batch stamps every entry
	/// it loads from one `UnixNow()`, so a reader comparing two entries of the same
	/// prefetch cannot see them arrive in different seconds because the loop took
	/// that long. It also makes the record a function of its inputs, which is what a
	/// test can pin.
	pub fn Arrived(Size:u64, ArrivedAtUnix:u64) -> LiveArrival {
		LiveArrival { ArrivedAtUnix, BytesArrived:Size, Size, EtaS:0.0 }
	}

	/// Chunked-arrival ETA estimate for a partial record: scale the elapsed time
	/// by the bytes still outstanding relative to what already arrived (linear
	/// arrival-rate model). Zero when nothing has arrived (no rate to
	/// extrapolate) or everything has.
	///
	/// The two zero cases are one predicate - "there is no outstanding ratio to
	/// compute" - so they answer through the same `None`, and the multiplication
	/// only runs when a rate exists.
	pub fn EtaEstimate(BytesArrived:u64, Size:u64, ElapsedS:f32) -> f32 {
		let Outstanding =
			(BytesArrived != 0 && BytesArrived < Size).then(|| (Size - BytesArrived) as f32 / BytesArrived as f32);

		Outstanding.map_or(0.0, |Ratio| ElapsedS * Ratio)
	}
}
