//! Poll-worker status is injected only while the worker is enabled.

use crate::{Flow::TurnContext::Fn::Fn as Assemble, State::Struct::AphroditeState};

#[test]
fn Disabled() {
	let mut s = AphroditeState::default();
	s.PollWorkerEnabled = false;
	s.TurnCounter = 5;
	crate::PollWorker::Insert::Fn::Fn(&mut s, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	let Context = Assemble(&mut s, None);
	assert!(
		!Context.contains("[poll workers]"),
		"disabled flag must omit poll worker status: {Context}"
	);
}

#[test]
fn Enabled() {
	let mut s = AphroditeState::default();
	s.PollWorkerEnabled = true;
	s.TurnCounter = 5;
	crate::PollWorker::Insert::Fn::Fn(&mut s, "t1".into(), "terminal".into(), "cargo build".into(), 1);
	let Context = Assemble(&mut s, None);
	assert!(
		Context.contains("[poll workers]"),
		"enabled flag must include poll worker status: {Context}"
	);
}
