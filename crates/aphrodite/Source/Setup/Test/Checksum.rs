//! The download checksum against a real GitHub release.
//!
//! One `#[ignore]`d test, kept here because it is the only thing that exercises
//! `Checksum::Fn` end to end - the unit-level behaviour is a hash comparison, but
//! the failure it protects against is a release whose sums file or asset entry is
//! missing, and only the published release can answer that.

use std::{fs, process::Command};

use crate::Setup::Dylib::Checksum::Fn as Checksum;

// Network-touching (real GitHub release) - not run by default, only on
// demand (`cargo test -p aphrodite --lib -- --ignored`) since the
// hermetic suite must stay network-free.
#[test]
#[ignore = "hits the real GitHub release - run explicitly to verify"]
fn TestVerifyDownloadChecksumAgainstRealRelease() {
	let Directory = std::env::temp_dir().join("aphrodite-checksum-test");
	fs::create_dir_all(&Directory).unwrap();
	let ReleaseDirectory = "https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v1.3.2";
	let Triple = "aarch64-apple-darwin";
	let Asset = "aphrodite-aarch64-apple-darwin";
	let Dest = Directory.join(Asset);
	let Status = Command::new("curl")
		.args(["-fsSL", "-o", Dest.to_str().unwrap(), &format!("{ReleaseDirectory}/{Asset}")])
		.status()
		.unwrap();
	assert!(Status.success());

	// Correct hash passes.
	Checksum(ReleaseDirectory, Triple, Asset, &Dest).expect("real asset must verify clean");

	// A deliberately corrupted file must be rejected, not silently accepted.
	fs::write(&Dest, b"corrupted content").unwrap();
	let Result = Checksum(ReleaseDirectory, Triple, Asset, &Dest);
	assert!(Result.is_err(), "corrupted asset must fail checksum verification");

	fs::remove_dir_all(&Directory).unwrap();
}
