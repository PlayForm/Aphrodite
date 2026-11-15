//! Waiting for the first OS shutdown signal.

/// Resolve when the process is asked to stop: Ctrl+C, or SIGTERM on unix.
///
/// Non-unix targets get a future that never resolves, so the second half of the
/// `select!` is a no-op there instead of a compile error. This deliberately does
/// not handle a *second* signal - that belongs to the drain step, which uses it
/// to force an abort.
pub async fn Signal() {
	let CtrlC = async {
		let _ = tokio::signal::ctrl_c().await;
	};
	#[cfg(unix)]
	let Terminate = async {
		match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
			Ok(mut s) => {
				s.recv().await;
			},
			Err(_) => {},
		}
	};
	#[cfg(not(unix))]
	let Terminate = std::future::pending::<()>();
	tokio::select! { _ = CtrlC => {}, _ = Terminate => {} }
}
