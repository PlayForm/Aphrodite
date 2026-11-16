//! The build stamp: a version is always present, and the stamp says how far past
//! a version it goes.

use crate::Health::Build::{Fn, Identity, Struct};

// ── The stamp is never empty (cargo always supplies the version) and it always
// states which KIND of identity it is. The second half is the seed's point: a
// health probe that reports a version and calls it a build cannot distinguish two
// images of the same version, so the payload has to say whether it can. ──
#[test]
fn probe_build_stamp_states_its_own_precision() {
	let Stamp = Fn::Fn();

	assert!(!Stamp.Version.is_empty(), "the crate version is always stamped");

	let JSON = Stamp.json();

	assert_eq!(
		JSON["version"].as_str(),
		Some(Stamp.Version),
		"the payload must carry the version the stamp carries"
	);

	let Identifies = JSON["identifies"].as_str().unwrap_or_default();

	assert_eq!(
		Identifies,
		match Stamp.Commit {
			Some(_) => "commit",
			None => "version-only",
		},
		"the payload must say which kind of identity the stamp is"
	);
}

// ── A stamp with no revision is Version, and one with a revision is Commit:
// the two identities are the enum's whole content, and a probe that reported
// Commit for an unstamped build would be the same lie as reporting a version as a
// build. ──
#[test]
fn probe_identity_follows_the_revision() {
	let Stamped = Struct::Struct {
		Version:"0.0.0",
		Commit:Some("abc1234def56-dirty"),
		Built:None,
		Target:None,
		Profile:None,
	};

	let Unstamped = Struct::Struct { Version:"0.0.0", Commit:None, Built:None, Target:None, Profile:None };

	assert!(
		matches!(Stamped.identity(), Identity::Enum::Enum::Commit),
		"a stamped revision is a build identity"
	);
	assert!(
		matches!(Unstamped.identity(), Identity::Enum::Enum::Version),
		"an unstamped build must not claim a revision identity"
	);
}
