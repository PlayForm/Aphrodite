//! The watcher thread body: come up, report readiness, then loop.

use std::{
	path::PathBuf,
	sync::{
		Arc,
		atomic::AtomicBool,
		mpsc::{Receiver, Sender, SyncSender},
	},
};

use crate::Session::Watcher::{Apply, Continue, Event, Outcome, Ready, Source, Wait};

/// Run the watcher until it is stopped, its channel closes, or no session wants
/// a reload any more.
///
/// Readiness is reported exactly once, before the loop: `Watching` when the
/// notifier is up, `Failed` with the reason when it is not. The spawner is
/// waiting on that message, which is why this leaf must never return before
/// sending one - a silent early return would leave the spawner blocked until
/// its own timeout and report a timeout instead of the real reason.
///
/// `Receiver` is the event channel (the notifier's filter feeds it), `Stop` is
/// the handle's stop flag, and `Wake` is a sender into that same channel so the
/// handle can produce a wake without a filesystem event.
pub(crate) fn Fn(
	Receiver:Receiver<()>,
	Stop:Arc<AtomicBool>,
	Wake:Sender<()>,
	ReadySender:SyncSender<Ready::Enum::Enum>,
) {
	// Watch the working directory first and the runtime home second - the same
	// precedence `Config::load()` resolves (issue 40: under a non-default
	// Hermes home the config lives in `<hermes-home>/aphrodite`, never in a
	// shadow `$HOME/.hermes/aphrodite`).
	let Paths:[PathBuf; 2] = [PathBuf::from("."), aphrodite::Home::Runtime::Fn::Fn()];

	// Bound, not discarded: dropping the notifier stops delivery, so it has to
	// live for as long as the loop does.
	let _Notifier = match Source::Fn::Fn(Wake, &Paths) {
		Ok(Notifier) => {
			let _ = ReadySender.send(Ready::Enum::Enum::Watching);

			Notifier
		},
		Err(Reason) => {
			let _ = ReadySender.send(Ready::Enum::Enum::Failed(Reason));

			return;
		},
	};

	// One blocking wake per burst, then one apply. `Wait` names the three ways
	// the loop can wake, `Continue` is the one place the "keep watching?"
	// policy lives, and `Step` composes them: it answers `None` when the loop is
	// over, so the body below is the pass itself.
	while let Some(Pass) = Step(&Receiver, &Stop) {
		// The reach, at debug level. A reload that reached three of five sessions
		// is a fact only this thread knows: the sweep counts it, and nobody else
		// ever asks. `skipped` is the wedge count, which is the difference between
		// "the file said stop" and "we could not look".
		match &Pass {
			Outcome::Enum::Enum::Applied(Report) => {
				tracing::debug!(
					target: crate::Debug::Init::Const::TARGET,
					applied = %Report.Applied,
					skipped = %Report.Skipped,
					wants_reload = %Report.WantsReload,
					"config reload applied"
				)
			},
			Outcome::Enum::Enum::Failed(Report) => {
				tracing::debug!(
					target: crate::Debug::Init::Const::TARGET,
					applied = %Report.Applied,
					skipped = %Report.Skipped,
					"config reload rejected (parse failure); previous values kept"
				)
			},
		}
	}
}

/// One pass: wait for the source, apply the file, then ask the policy whether to
/// keep watching.
///
/// `None` is the loop's exit - the source stopped or closed, or no session wants a
/// reload any more - so the loop has one condition to obey instead of two.
fn Step(Receiver:&Receiver<()>, Stop:&Arc<AtomicBool>) -> Option<Outcome::Enum::Enum> {
	match Wait::Fn::Fn(Receiver, Stop) {
		Event::Enum::Enum::Tick => {
			let Pass = Apply::Fn::Fn(&aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Load());

			Continue::Fn::Fn(&Pass).then_some(Pass)
		},
		Event::Enum::Enum::Stop | Event::Enum::Enum::Closed => None,
	}
}
