//! Collapsing a burst of events into one tick.

/// Wait for writes to settle, then discard everything that arrived during the
/// wait.
///
/// The 500 ms window is the contract: an editor that writes a file in three
/// passes, or a tool that truncates then fills it, produces three events that
/// describe one change. Draining afterwards is what makes the caller's reload
/// run once per burst rather than once per event, and it is the reason this
/// window is not a "wait for quiet" loop - a directory that never goes quiet
/// would then never reload.
///
/// `try_recv` is not called one extra time: `from_fn` stops calling the closure
/// after it returns `None`, so the channel is left exactly where the burst ended.
pub(crate) async fn Fn(Rx:&mut tokio::sync::mpsc::Receiver<()>) {
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	std::iter::from_fn(|| Rx.try_recv().ok()).for_each(drop);
}
