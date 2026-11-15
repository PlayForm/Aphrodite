//! Building and installing the subscriber.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::Logging::Level::Fn::Fn as ResolveFilter;

/// Build the registry and install it as the global dispatcher.
///
/// Called once, from `Logging::Fn`; never call it directly. A second
/// `try_init` fails, and a caller that saw that error would report a startup
/// failure for what is really "a subscriber is already installed".
///
/// `Compact` keeps the operator-facing single-line format (no timestamp, no
/// target) that the `--log-compact` flag and `APHRODITE_LOG_COMPACT` select;
/// the structured format is the default because a log a machine reads needs
/// its fields and its time.
pub(crate) fn Fn(Compact:bool, Configured:Option<tracing::Level>) -> anyhow::Result<()> {
	let Filter = ResolveFilter(Configured);
	let Subscriber = tracing_subscriber::registry().with(Filter);
	match Compact {
		true => {
			Subscriber
				.with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
				.try_init()?;
		},
		false => {
			Subscriber.with(tracing_subscriber::fmt::layer()).try_init()?;
		},
	}
	Ok(())
}
