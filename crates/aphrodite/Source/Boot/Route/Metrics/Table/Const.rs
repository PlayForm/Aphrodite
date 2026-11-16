//! The exposition, in the order it is written.

use crate::Boot::Route::Metrics::{Entry, Kind, Label, Piece};

/// The label set every counter that is not broken down by mode carries.
const PLAIN:Label::Enum::Enum = Label::Enum::Enum::Plain;

/// The label set of the two request counters: the proxy mode, `unknown` when the
/// document does not name one.
const MODE:Label::Enum::Enum = Label::Enum::Enum::Document { Key:"mode", Path:&["mode"], Fallback:"unknown" };

/// One counter piece, spelled out once.
const fn Counter(
	Name:&'static str,
	Path:&'static [&'static str],
	Kind:Kind::Enum::Enum,
	Label:Label::Enum::Enum,
) -> Piece::Enum::Enum {
	Piece::Enum::Enum::Entry(Entry::Struct::Struct { Name, Path, Kind, Label })
}

/// The exposition, in the order Prometheus reads it.
///
/// This list is the whole answer to "which metrics exist": adding a counter is
/// one line here, and nothing else in the module changes. The order is the
/// document's own history - counters first, then the histogram, then the
/// per-section breakdowns - and is kept so a scrape diff stays empty.
pub const ORDER:[Piece::Enum::Enum; 27] = [
	Counter(
		"aphrodite_requests_total",
		&["requests", "total"],
		Kind::Enum::Enum::Shown,
		MODE,
	),
	Counter(
		"aphrodite_requests_compressed_total",
		&["requests", "compressed"],
		Kind::Enum::Enum::Shown,
		MODE,
	),
	Counter(
		"aphrodite_tokens_saved_total",
		&["tokens_saved"],
		Kind::Enum::Enum::Shown,
		PLAIN,
	),
	Counter("aphrodite_ccr_hits_total", &["ccr", "hits"], Kind::Enum::Enum::Shown, PLAIN),
	Counter("aphrodite_ccr_misses_total", &["ccr", "misses"], Kind::Enum::Enum::Shown, PLAIN),
	Counter(
		"aphrodite_ccr_created_total",
		&["ccr", "created"],
		Kind::Enum::Enum::Shown,
		PLAIN,
	),
	Counter(
		"aphrodite_tool_relay_calls_total",
		&["tool_relay_calls"],
		Kind::Enum::Enum::Shown,
		PLAIN,
	),
	Counter("aphrodite_cache_hits_total", &["cache", "hits"], Kind::Enum::Enum::Count, PLAIN),
	Counter(
		"aphrodite_cache_misses_total",
		&["cache", "misses"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Piece::Enum::Enum::Histogram,
	Counter(
		"aphrodite_compression_ratio_ema",
		&["compression_ratio_ema"],
		Kind::Enum::Enum::Ratio,
		PLAIN,
	),
	Counter(
		"aphrodite_inline_ccr_hits_total",
		&["inline_ccr", "hits"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_inline_ccr_misses_total",
		&["inline_ccr", "misses"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_tool_relay_success_total",
		&["tool_relay", "success"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_tool_relay_failure_total",
		&["tool_relay", "failure"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_notify_success_total",
		&["notify", "success"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_notify_failure_total",
		&["notify", "failure"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_upstream_errors_total",
		&["upstream_errors", "4xx"],
		Kind::Enum::Enum::Count,
		Label::Enum::Enum::Literal { Key:"code", Value:"4xx" },
	),
	Counter(
		"aphrodite_upstream_errors_total",
		&["upstream_errors", "5xx"],
		Kind::Enum::Enum::Count,
		Label::Enum::Enum::Literal { Key:"code", Value:"5xx" },
	),
	Counter(
		"aphrodite_upstream_timeouts_total",
		&["upstream_errors", "timeouts"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_upstream_connect_errors_total",
		&["upstream_errors", "connect_errors"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_sse_stream_errors_total",
		&["upstream_errors", "sse_stream_errors"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_ccr_store_entries",
		&["ccr_store", "entries"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_ccr_store_bytes",
		&["ccr_store", "bytes_approx"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_request_body_bytes_total",
		&["body_bytes", "request"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_response_body_bytes_total",
		&["body_bytes", "response"],
		Kind::Enum::Enum::Count,
		PLAIN,
	),
	Counter(
		"aphrodite_upstream_latency_seconds_total",
		&["upstream_latency_micros"],
		Kind::Enum::Enum::Seconds,
		PLAIN,
	),
];
