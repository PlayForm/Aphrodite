//! Draining the listeners after the first shutdown signal.

use tokio::{sync::watch::Sender, task::JoinHandle};
use Boot::Listen::Shutdown::Signal;

/// Wait for a shutdown signal, then drain every listener.
///
/// Three outcomes race: every listener finishes gracefully, the 5-second drain
/// timeout expires, or a second signal arrives. The latter two abort whatever is
/// still running. `DrainFut` must never be re-awaited after the `select!` - it
/// was polled there, and re-awaiting a pinned future that already completed is
/// double-poll UB; the abort handles are what stop the stragglers.
pub async fn Drain(ShutdownTx:Sender<bool>, Handles:Vec<JoinHandle<()>>) {
	// Wait for first shutdown signal - triggers graceful shutdown in all proxies
	Signal().await;
	let _ = ShutdownTx.send(true);
	tracing::info!("shutdown signal received, draining in-flight requests...");

	// Clone abort handles so we can force-kill after handles are moved into
	// join_all
	let AbortHandles:Vec<_> = Handles.iter().map(|h| h.abort_handle()).collect();

	// Listen for a second Ctrl+C to force immediate shutdown
	let SecondSignal = async {
		let _ = tokio::signal::ctrl_c().await;
		tracing::warn!("second shutdown signal received, forcing immediate shutdown");
	};
	tokio::pin!(SecondSignal);

	// 5-second drain timeout before forcing abort
	let DrainTimeout = tokio::time::sleep(std::time::Duration::from_secs(5));
	tokio::pin!(DrainTimeout);

	// Wait for graceful drain (via axum's with_graceful_shutdown), second signal,
	// or timeout. NOTE: do NOT re-await drain_fut after select! - it was polled by
	// select! and re-awaiting would be double-poll UB. Abort handles handle the
	// remaining tasks.
	let DrainFut = futures::future::join_all(Handles);
	tokio::pin!(DrainFut);

	tokio::select! {
		_ = &mut DrainFut => {
			tracing::info!("all proxy listeners completed gracefully");
		}
		_ = &mut DrainTimeout => {
			tracing::info!("drain timeout (5s) reached, aborting remaining tasks");
			for h in &AbortHandles {
				h.abort();
			}
		}
		_ = &mut SecondSignal => {
			tracing::info!("force shutdown on second signal, aborting remaining tasks");
			for h in &AbortHandles {
				h.abort();
			}
		}
	}
}
