//! The rolling turn window and its aggregate.

use crate::{
	Flow::Window::Turn::Fn::Fn as Window,
	State::{Event::Struct::Struct as ToolEvent, Struct::AphroditeState},
};

#[test]
fn Aggregate() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 10;
	s.RecordEvent(ToolEvent {
		Turn:10,
		Tool:"read_file".into(),
		Sig:1,
		Okay:true,
		ErrorSig:None,
		bytes:100,
		WrotePath:None,
	});
	s.RecordEvent(ToolEvent {
		Turn:10,
		Tool:"write_file".into(),
		Sig:2,
		Okay:true,
		ErrorSig:None,
		bytes:50,
		WrotePath:Some("src/a.rs".into()),
	});
	s.RecordEvent(ToolEvent {
		Turn:9,
		Tool:"terminal".into(),
		Sig:3,
		Okay:false,
		ErrorSig:Some(42),
		bytes:20,
		WrotePath:None,
	});
	// old event outside the window
	s.RecordEvent(ToolEvent {
		Turn:2,
		Tool:"read_file".into(),
		Sig:4,
		Okay:true,
		ErrorSig:None,
		bytes:0,
		WrotePath:None,
	});
	let w = Window(&s, 5);
	assert_eq!(w.Reads, 1);
	assert_eq!(w.Writes, 1);
	assert_eq!(w.NewFiles, 1);
	assert_eq!(w.Errors, 1);
	assert_eq!(w.Distinct, 1);
	assert_eq!(w.Total, 3, "the turn-2 event is outside the 5-turn window");
}
