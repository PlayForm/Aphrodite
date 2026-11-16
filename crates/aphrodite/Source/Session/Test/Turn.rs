//! The turn counter advances and the scan cursor resets.

use crate::{Session::Turn::Fn::Fn as Turn, State::Struct::AphroditeState};

#[test]
fn Advance() {
	let mut s = AphroditeState::default();
	assert_eq!(Turn(&mut s), 1);
	assert_eq!(Turn(&mut s), 2);
	assert_eq!(s.ScannedMessageIndex, 0);
}
