//! The conversation index: eviction at the cap, and read order.

use crate::{
	Session::{Archive::Fn::Fn as Archive, Index::Fn::Fn as Index},
	State::Struct::AphroditeState,
};

#[test]
fn Eviction() {
	let mut s = AphroditeState::default();
	(0..55).for_each(|i| {
		Archive(&mut s, &format!("hash{}", i), "summary", 100);
		s.TurnCounter += 1;
	});
	assert!(s.ConvIndex.len() <= 50);
	assert!(!s.ConvIndex.contains_key(&1));
}

#[test]
fn Rewind() {
	// A rewound turn counter makes the new entry the oldest key. Evicting "the
	// oldest" would then delete the archive this call exists to record and keep the
	// entries it should have displaced.
	let mut s = AphroditeState::default();
	(0..50).for_each(|i| {
		s.TurnCounter = i;
		Archive(&mut s, &format!("hash{i}"), "summary", 100);
	});
	assert_eq!(s.ConvIndex.len(), 50);

	// Rewind: turn 0 is already present, so this overwrites rather than grows...
	s.TurnCounter = 0;
	Archive(&mut s, "rewound", "summary", 100);
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
		Archive(&mut s, &format!("hash{i}"), "summary", 100);
	});
	s.TurnCounter = 0;
	Archive(&mut s, "turn-zero", "summary", 100);
	assert_eq!(s.ConvIndex.len(), 50);
	assert_eq!(
		s.ConvIndex.get(&0).map(|(Hash, ..)| Hash.as_str()),
		Some("turn-zero"),
		"the entry just written must survive its own eviction pass"
	);
	assert!(!s.ConvIndex.contains_key(&1), "the oldest other key goes instead");
}

#[test]
fn Sorted() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 2;
	Archive(&mut s, "hash2", "second", 20);
	s.TurnCounter = 1;
	Archive(&mut s, "hash1", "first", 10);
	let Turns = Index(&s);
	assert_eq!(Turns.len(), 2);
	assert_eq!(Turns[0]["turn"], 1);
	assert_eq!(Turns[1]["turn"], 2);
}
