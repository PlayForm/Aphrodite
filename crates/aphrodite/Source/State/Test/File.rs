//! The referenced-files list: de-duplication and tool-name refresh.

use crate::State::Struct::AphroditeState;

#[test]
fn TestRecordFileDedup() {
	let mut s = AphroditeState::default();
	s.RecordFile("/tmp/a".into(), "read".into());
	s.RecordFile("/tmp/a".into(), "write".into());
	assert_eq!(s.ReferencedFiles.len(), 1);
	assert_eq!(s.ReferencedFiles[0].1, "write"); // Updated tool
}
