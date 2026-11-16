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
		// The registration is the decision: a handler the OS installed yields the
		// signal to wait on. One it refused leaves nothing to wait for, so this half
		// of the `select!` resolves at once and the shutdown proceeds - the shape the
		// non-unix arm has, where there is likewise no signal.
		//
		// The match stays a match: `if let` is the branch form the house style keeps
		// out of the tree, and the empty arm is the whole point of the shape.
		#[allow(clippy::single_match)]
		match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
			Ok(mut s) => {
				s.recv().await;
			},
			Err(_) => (),
		}
	};
	#[cfg(not(unix))]
	let Terminate = std::future::pending::<()>();
	tokio::select! { _ = CtrlC => {}, _ = Terminate => {} }
}
