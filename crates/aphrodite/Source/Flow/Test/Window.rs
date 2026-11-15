//! The rolling turn window and its aggregate.

use crate::{
	Flow::TurnWindow,
	State::{AphroditeState, ToolEvent},
};

#[test]
fn TestTurnWindowCountsReadsWritesErrors() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 10;
	s.RecordToolEvent(ToolEvent {
		Turn:10,
		Tool:"read_file".into(),
		Sig:1,
		Okay:true,
		ErrorSig:None,
		bytes:100,
		WrotePath:None,
	});
	s.RecordToolEvent(ToolEvent {
		Turn:10,
		Tool:"write_file".into(),
		Sig:2,
		Okay:true,
		ErrorSig:None,
		bytes:50,
		WrotePath:Some("src/a.rs".into()),
	});
	s.RecordToolEvent(ToolEvent {
		Turn:9,
		Tool:"terminal".into(),
		Sig:3,
		Okay:false,
		ErrorSig:Some(42),
		bytes:20,
		WrotePath:None,
	});
	// old event outside the window
	s.RecordToolEvent(ToolEvent {
		Turn:2,
		Tool:"read_file".into(),
		Sig:4,
		Okay:true,
		ErrorSig:None,
		bytes:0,
		WrotePath:None,
	});
	let w = TurnWindow(&s, 5);
	assert_eq!(w.Reads, 1);
	assert_eq!(w.Writes, 1);
	assert_eq!(w.NewFiles, 1);
	assert_eq!(w.Errors, 1);
	assert_eq!(w.DistinctErrorSigs, 1);
	assert_eq!(w.TotalCalls, 3, "the turn-2 event is outside the 5-turn window");
}
