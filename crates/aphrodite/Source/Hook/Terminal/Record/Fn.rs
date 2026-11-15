//! Record a terminal call's telemetry event.

use crate::{
	Flow,
	State::{AphroditeState, ToolEvent},
};

/// Record the telemetry event for one terminal call (P2/T6).
///
/// `returncode == 0` (or an absent code) is `Okay`. Terminal failures carry no
/// `ErrorType`, so the signature keys on the command plus the first
/// error-looking line of the output - that is what lets two different failures
/// of the same command count as different failures.
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, command:Option<&str>, Returncode:Option<i64>) {
	let Okay = Returncode.map(|Rc| Rc == 0).unwrap_or(true);
	let Args = command.map(|c| serde_json::json!({"command": c}));
	let Sig = Flow::NormalizeArgsSig("terminal", Args.as_ref());
	let ErrorSig = if Okay {
		None
	} else {
		// Terminal failures have no error_type; key on the command + first
		// error-looking line of the output so distinct failures differ.
		let FirstError = Content
			.lines()
			.find(|l| l.contains("error") || l.contains("Error") || l.contains("FAILED") || l.contains("panicked"))
			.or_else(|| Content.lines().next());
		Some(Flow::ErrorSig(command, FirstError))
	};
	State.RecordToolEvent(ToolEvent {
		Turn:State.TurnCounter,
		Tool:"terminal".to_string(),
		Sig,
		Okay,
		ErrorSig,
		bytes:Content.len(),
		WrotePath:None,
	});
}
