//! The referenced-files list: de-duplication and tool-name refresh.

use crate::State::Struct::AphroditeState;

#[test]
fn Dedup() {
	let mut s = AphroditeState::default();
	s.File.Record("/tmp/a".into(), "read".into());
	s.File.Record("/tmp/a".into(), "write".into());
	assert_eq!(s.File.Recent.len(), 1);
	assert_eq!(s.File.Recent[0].1, "write"); // Updated tool
}
