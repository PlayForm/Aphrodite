//! The single clock shared by the prefetch and poll-worker surfaces.
//!
//! Both surfaces stamp arrival times, and a task created by the poll worker
//! is compared against a record written by the prefetch path. Two `SystemTime`
//! call sites would drift apart by a second at the wrong moment and make a
//! fresh task look already-elapsed, so there is exactly one here.

/// Unix-seconds now - single source of truth for live-container arrival
/// timestamps so the prefetch and poll-worker surfaces share one clock.
pub fn Fn() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_secs())
		.unwrap_or(0)
}
