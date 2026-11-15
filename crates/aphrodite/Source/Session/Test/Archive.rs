//! The conversation index: eviction at the cap, and read order.

use crate::{
	Session::{ArchiveTurn, GetConvIndex},
	State::AphroditeState,
};

#[test]
fn TestArchiveEviction() {
	let mut s = AphroditeState::default();
	for i in 0..55 {
		ArchiveTurn(&mut s, &format!("hash{}", i), "summary", 100);
		s.TurnCounter += 1;
	}
	assert!(s.ConvIndex.len() <= 50);
	assert!(!s.ConvIndex.contains_key(&1));
}

#[test]
fn TestGetConvIndexSortedOldestFirst() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 2;
	ArchiveTurn(&mut s, "hash2", "second", 20);
	s.TurnCounter = 1;
	ArchiveTurn(&mut s, "hash1", "first", 10);
	let Turns = GetConvIndex(&s);
	assert_eq!(Turns.len(), 2);
	assert_eq!(Turns[0]["turn"], 1);
	assert_eq!(Turns[1]["turn"], 2);
}
