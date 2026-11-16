//! Waiting for a burst of writes to settle.

use std::time::Duration;

use crate::Boot::Listen::Watcher::Target;
use super::Outcome;

/// How long the file has to stay quiet before a burst counts as finished.
const Quiet:Duration = Duration::from_millis(500);

/// The longest a burst may hold a reload back, measured from its first event.
///
/// A pure quiet-period debounce starves on a file that is written continuously:
/// every new event restarts the window, so the reload never happens. The ceiling is
/// what bounds that - the reload runs at 2 s even if the writes never stop.
const Ceiling:Duration = Duration::from_secs(2);

/// Wait for the burst to settle, then report how it ended.
///
/// **The contract is a trailing-edge quiet period, capped.** A reload runs after
/// 500 ms in which no further event arrived, or at 2 s from the first event,
/// whichever comes first. The earlier shape slept a fixed 500 ms from the first
/// event and then discarded whatever had arrived meanwhile: an editor that writes a
/// file in three passes spread over more than half a second produced a reload against
/// a half-written file, and a directory with steady churn reloaded in the middle of
/// it rather than after it.
///
/// A quiet period is the contract a reload actually needs, because what it wants is
/// the *finished* file rather than the first byte of it. The ceiling is what keeps
/// "wait for quiet" from becoming "wait forever on a busy directory".
///
/// Events consumed while waiting are dropped deliberately: each one only means "the
/// file may have changed", and the reload that follows reads the file as it is now,
/// so N events describe one outcome.
pub(crate) async fn Fn(Rx:&mut tokio::sync::mpsc::Receiver<Target::Struct::Struct>) -> Outcome::Enum::Enum {
	let First = tokio::time::Instant::now();

	loop {
		// The ceiling is a budget, and a spent ceiling is the absence of a window
		// rather than a zero-length one: a zero window would still poll the channel
		// once and consume an event the overdue reload is about to account for.
		let Left = Ceiling.saturating_sub(First.elapsed());
		let Budget = (!Left.is_zero()).then_some(Left);

		// The ceiling has already expired: the reload is overdue, and waiting any
		// longer would let a directory that never goes quiet starve it.
		let Step = match Budget {
			None => return Outcome::Enum::Enum::Settled,
			Some(Remaining) => tokio::time::timeout(Quiet.min(Remaining), Rx.recv()).await,
		};

		match Step {
			// Another write landed inside the window: the file is still being
			// written, so the quiet window restarts from now.
			Ok(Some(_)) => continue,
			// The source closed mid-burst: this is the last burst, and the caller
			// still has a pending change to apply.
			Ok(None) => return Outcome::Enum::Enum::Closed,
			// Quiet for the full window - or the ceiling cut the window short.
			Err(_) => return Outcome::Enum::Enum::Settled,
		}
	}
}
