//! How many worker threads the proxy runtime gets.

/// The worker-thread count for the Tokio runtime.
///
/// The default is `4 × CPU`, floored at 32: this is an I/O-bound proxy, so it
/// needs far more threads than it has cores, and a small machine still needs
/// enough to keep several upstream requests in flight.
///
/// `APHRODITE_WORKER_THREADS` overrides it. A value that is present but does not
/// parse warns and uses the computed default rather than silently falling back -
/// silently ignoring a typo'd override leaves the operator unable to tell "my
/// override didn't apply" from "I didn't set one".
///
/// The warning goes to the stderr sink, not to `tracing`: the pool is built
/// before `Boot::Run` parses the configuration, so no subscriber exists yet and a
/// `tracing::warn!` would be dropped.
pub(crate) fn Fn() -> usize {
	let ComputedDefault = || (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8) * 4).max(32);

	std::env::var("APHRODITE_WORKER_THREADS")
		.ok()
		.map_or_else(ComputedDefault, |Raw| {
			Raw.parse().unwrap_or_else(|_| {
				aphrodite::Logging::Stderr::Fn::Fn(&format!(
					"APHRODITE_WORKER_THREADS={Raw:?} is not a valid number; using the computed default"
				));

				ComputedDefault()
			})
		})
}
