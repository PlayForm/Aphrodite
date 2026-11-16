//! The poll marker's shape: it must be a resolvable CCR marker typed `Poll`
//! and must embed the task hash, never an output hash.

use crate::{PollWorker::Marker::Fn::Fn as Marker, State::Identity::Command::Struct::Struct as Command};

#[test]
fn Shape() {
	let Marker = Marker(&Command::from("abc123def456"), "cargo build", 5);
	assert!(Marker.contains("<<<CCR:"));
	assert!(Marker.contains("poll"));
	assert!(Marker.contains("cargo build"));
	assert!(Marker.contains("turn 5"));
}

#[test]
fn Valid() {
	let Marker = Marker(
		&Command::from("abc123def456abc123def456abc123def456abc123"),
		"cargo build --release",
		5,
	);
	// Must start with the preview format
	assert!(Marker.contains("[poll:running"), "marker must contain preview");
	// Must end with the CCR marker
	assert!(Marker.contains("<<<CCR:"), "marker must contain CCR tag");
	// Must contain the hash
	assert!(Marker.contains("abc123def456"), "marker must contain hash");
	// Must contain the type
	assert!(Marker.contains("|poll|"), "marker must contain poll type");
}
