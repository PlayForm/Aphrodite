//! Writing a reloaded configuration into every live listener.

use std::sync::{Arc, atomic::Ordering};

use aphrodite::{
	Configuration::Proxy::Multi::Struct::Struct as MultiConfiguration,
	Proxy,
	Proxy::State::Struct::Struct as AppState,
};

use crate::Boot::Listen::Watcher::Target;

/// Load the target file and apply its compression thresholds to every listener.
///
/// One table drives every listener (report 07 F2/T15): the document carries one
/// top-level `[compression]` section, and each listener resolves its own cache-vs-token
/// threshold from it. That is why this takes the whole list of states rather than one -
/// a file change is a process-wide fact.
///
/// A load that fails only warns and leaves the previous thresholds in force. Reverting
/// to built-in defaults on a transient read failure would silently change compression
/// behaviour on a file that is merely mid-write, which is the opposite of what hot
/// reload is for.
///
/// The file comes from the event's target rather than from a parameter, so the path
/// this loads is the path that was watched - resolved once, not re-derived here from a
/// string that travelled beside the channel.
///
/// The stores are `Relaxed` on purpose: each threshold is an independent value read
/// once per request, so there is no ordering relationship between them that a stronger
/// ordering would establish.
pub(crate) fn Fn(States:&[Arc<AppState>], Target:&Target::Struct::Struct) {
	let Configuration = match MultiConfiguration::Load(&Target.File) {
		Ok(Configuration) => Configuration,
		Err(e) => {
			tracing::warn!(error = %e, path = %Target.File.display(), "failed to reload config on file change");

			return;
		},
	};

	let Thresholds = Proxy::Threshold::Resolve::Fn::Fn(Configuration.Compression.as_ref());

	States.iter().for_each(|State| {
		State.CacheThreshold.store(Thresholds.Cache, Ordering::Relaxed);
		State.TokenThreshold.store(Thresholds.Token, Ordering::Relaxed);
		State.InlineThreshold.store(Thresholds.inline, Ordering::Relaxed);
		State
			.CodeMultiplier
			.store((Thresholds.CodeMultiplier * 100.0) as u64, Ordering::Relaxed);
	});

	tracing::info!(
		path = %Target.File.display(),
		CacheThreshold = Thresholds.Cache,
		TokenThreshold = Thresholds.Token,
		InlineThreshold = Thresholds.inline,
		CodeMultiplier = Thresholds.CodeMultiplier,
		Listeners = States.len(),
		"config reloaded - compression thresholds applied to all live listeners"
	);
}
