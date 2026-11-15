//! The turn counter advances and the scan cursor resets.

use crate::{Session::NextTurn, State::AphroditeState};

#[test]
fn TestNextTurn() {
	let mut s = AphroditeState::default();
	assert_eq!(NextTurn(&mut s), 1);
	assert_eq!(NextTurn(&mut s), 2);
	assert_eq!(s.ScannedMessageIndex, 0);
}
