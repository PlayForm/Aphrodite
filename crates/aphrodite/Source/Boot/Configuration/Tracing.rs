//! Installing the tracing subscriber.
//!
//! Runs once, after the config is parsed and before anything that logs. The
//! filter comes from `RUST_LOG` when set and falls back to `info`; `LogCompact`
//! selects the single-line, timestamp-free format a terminal operator wants over
//! the default structured one.

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Install the global tracing subscriber.
///
/// Returns `Err` when a subscriber is already installed - the caller must treat
/// that as fatal rather than continuing, because the alternative is running with
/// whatever subscriber won and silently losing every diagnostic this process
/// would otherwise emit.
pub fn Subscribe(LogCompact:bool) -> anyhow::Result<()> {
	let Filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	let Subscriber = tracing_subscriber::registry().with(Filter);
	if LogCompact {
		Subscriber
			.with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
			.try_init()?;
	} else {
		Subscriber.with(tracing_subscriber::fmt::layer()).try_init()?;
	}
	Ok(())
}
