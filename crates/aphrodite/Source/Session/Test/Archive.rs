//! The conversation index: eviction at the cap, and read order.

use crate::{
	Session::{Archive::Fn::ArchiveTurn, Index::Fn::GetConvIndex},
	State::Struct::AphroditeState,
};

#[test]
fn TestArchiveEviction() {
	let mut s = AphroditeState::default();
	(0..55).for_each(|i| {
		ArchiveTurn(&mut s, &format!("hash{}", i), "summary", 100);
		s.TurnCounter += 1;
	});
	assert!(s.ConvIndex.len() <= 50);
	assert!(!s.ConvIndex.contains_key(&1));
}

#[test]
fn TestArchiveTurnKeepsTheEntryItJustWroteWhenTheTurnRewinds() {
	// A rewound turn counter makes the new entry the oldest key. Evicting "the
	// oldest" would then delete the archive this call exists to record and keep the
	// entries it should have displaced.
	let mut s = AphroditeState::default();
	(0..50).for_each(|i| {
		s.TurnCounter = i;
		ArchiveTurn(&mut s, &format!("hash{i}"), "summary", 100);
	});
	assert_eq!(s.ConvIndex.len(), 50);

	// Rewind: turn 0 is already present, so this overwrites rather than grows...
	s.TurnCounter = 0;
	ArchiveTurn(&mut s, "rewound", "summary", 100);
	assert_eq!(s.ConvIndex.len(), 50, "an overwrite does not overfill the index");
	assert_eq!(
		s.ConvIndex.get(&0).map(|(Hash, ..)| Hash.as_str()),
		Some("rewound"),
		"the rewritten entry survives"
	);

	// ...and a rewound turn that is NEW (a gap key below every existing one) grows
	// the index over its cap: the eviction must take another key, not this one.
	let mut s = AphroditeState::default();
	(1..=50).for_each(|i| {
		s.TurnCounter = i;
		ArchiveTurn(&mut s, &format!("hash{i}"), "summary", 100);
	});
	s.TurnCounter = 0;
	ArchiveTurn(&mut s, "turn-zero", "summary", 100);
	assert_eq!(s.ConvIndex.len(), 50);
	assert_eq!(
		s.ConvIndex.get(&0).map(|(Hash, ..)| Hash.as_str()),
		Some("turn-zero"),
		"the entry just written must survive its own eviction pass"
	);
	assert!(!s.ConvIndex.contains_key(&1), "the oldest other key goes instead");
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
