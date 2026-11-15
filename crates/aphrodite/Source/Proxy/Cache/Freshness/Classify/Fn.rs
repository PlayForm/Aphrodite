//! Fresh or expired, decided from the values passed in.
//!
//! `Now` is an argument, not `Instant::now()` read inside: a classifier that
//! reads the clock cannot be reasoned about, and the caller that has to evict
//! or promote is the one that should decide which instant the whole lookup
//! agrees on. `saturating_duration_since` rather than `duration_since`, because
//! an entry stamped after `Now` is a clock anomaly to classify, not a panic.

use std::time::{Duration, Instant};

use crate::Proxy::Cache::Freshness::Enum::Enum as Freshness;

/// Is an entry stored at `StoredAt` still inside `Ttl` at `Now`?
pub(crate) fn Fn(StoredAt:Instant, Now:Instant, Ttl:Duration) -> Freshness {
	if Now.saturating_duration_since(StoredAt) < Ttl {
		Freshness::Fresh
	} else {
		Freshness::Expired
	}
}
