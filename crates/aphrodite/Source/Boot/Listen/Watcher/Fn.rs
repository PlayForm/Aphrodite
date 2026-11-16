//! The production wiring: the only part of the watcher that needs a real filesystem
//! watch.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;

use super::{Loop, Outcome, Source, Target};
use crate::Boot::State::Listener::Bound;

/// Watch the configuration file and apply threshold changes to every listener.
///
/// The watch is created here, before the task is spawned, so its readiness is a fact
/// this function returns instead of a warning emitted from a task the caller never
/// holds. A watch that cannot start leaves the process running on the configuration it
/// already loaded - hot reload is a convenience, and refusing to start because `notify`
/// failed would be a worse outcome than needing a restart - but it is reported, so
/// "reload does not work here" is never a mystery.
///
/// Everything below this function is free of `notify`, which is what lets `Loop` be
/// driven from a channel a test owns.
pub fn Fn(ConfigurationPath:&str, Bound:&[Bound]) -> Outcome::Enum::Enum {
	let Target = Target::Struct::Struct::new(ConfigurationPath);
	let States:Vec<Arc<AppState>> = Bound.iter().map(|b| b.State.clone()).collect();

	let Source = match Source::Fn::Fn(&Target) {
		Ok(Source) => Source,
		Err(Cause) => {
			tracing::warn!(%Cause, "config hot reload is inactive - a config change needs a restart");

			return Outcome::Enum::Enum::Inactive;
		},
	};

	tracing::info!(path = %Target.File.display(), "config file watcher active");

	Outcome::Enum::Enum::Watching(tokio::spawn(async move {
		Loop::Fn::Fn(Source, States).await;
	}))
}
