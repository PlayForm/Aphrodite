//! Spawning one serving task per bound listener.

use tokio::{sync::watch::Receiver, task::JoinHandle};

use crate::Boot::{Listen::Serve::Serve, State::Listener::Bound};

/// Spawn a serving task per listener and return their join handles.
///
/// Takes ownership of the shutdown receiver and drops it on return, so the
/// receivers cloned into each task see the channel close once the last one is
/// gone - that close is what wakes a listener whose sender was never asked to
/// send `true`. A serve failure is logged by the task itself and does not
/// propagate: one dead listener must not take the process down while its
/// siblings are still answering.
pub fn Spawn(Bound:Vec<Bound>, ShutdownRx:Receiver<bool>) -> Vec<JoinHandle<()>> {
	let mut Handles = Vec::new();
	for b in Bound {
		let Rx = ShutdownRx.clone();
		let Handle = tokio::spawn(async move {
			if let Err(e) = Serve(b, Rx).await {
				tracing::error!(%e, "proxy listener failed");
			}
		});
		Handles.push(Handle);
	}
	// Drop the original sender so run_single receivers get a RecvError::Closed
	// when main() finishes (SIGTERM alternative: drop triggers changed() wakeup).
	drop(ShutdownRx);
	Handles
}
