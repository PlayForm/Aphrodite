//! Resolving the filter from the environment and the configuration.

use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Build the `EnvFilter`: `APHRODITE_LOG`, then `RUST_LOG`, then `Configured`,
/// then `info`.
///
/// `try_from_env` (not `from_env`) on purpose: a malformed `APHRODITE_LOG` must
/// fall through to the next source instead of aborting startup, and a
/// malformed `RUST_LOG` must fall through to the configured level for the same
/// reason. `Configured` is the caller's own level (`--dev` means `debug`), and
/// `Level::INFO` is the final answer when nobody said anything.
///
/// The filter is not a second gate. A `tracing` event still costs one filter
/// check at the callsite, so a `debug!` inside the per-turn pipeline stays off
/// at the default level - but a span or event the filter rejects never reaches
/// the fmt layer, so nothing is formatted for it either.
pub(crate) fn Fn(Configured:Option<Level>) -> EnvFilter {
	// Each source is tried only when the previous one declined, and the final
	// level is built only when no source produced a filter.
	EnvFilter::try_from_env("APHRODITE_LOG")
		.or_else(|_| EnvFilter::try_from_default_env())
		.unwrap_or_else(|_| EnvFilter::new(Configured.unwrap_or(Level::INFO).as_str()))
}
