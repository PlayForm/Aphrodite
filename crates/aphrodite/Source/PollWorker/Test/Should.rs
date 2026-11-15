//! Backgrounding decisions: the size floor, the prefix table and its
//! boundaries.

use crate::PollWorker::Should::{Command::Prefix::Const::SlowCommandPrefixes, Entry::Fn::ShouldBackground};

#[test]
fn TestShouldBackgroundSlowCommand() {
	let Result = ShouldBackground(&"x".repeat(5000), Some("cargo build --release"));
	assert!(Result.is_some(), "cargo build should be backgrounded");
}

#[test]
fn TestShouldNotBackgroundSmallOutput() {
	let Result = ShouldBackground("short output", Some("cargo build"));
	assert!(Result.is_none(), "small output should not be backgrounded");
}

#[test]
fn TestShouldBackgroundLargeUnknownOutput() {
	// Unknown command that doesn't match a slow prefix - passes through.
	let Result = ShouldBackground(&"x".repeat(10000), Some("some-slow-script.sh"));
	assert!(
		Result.is_none(),
		"unknown command should NOT be backgrounded (slow-prefix check)"
	);
}

#[test]
fn TestShouldNotBackgroundEmptyCommandSmallOutput() {
	let Result = ShouldBackground("small", None);
	assert!(Result.is_none(), "no command + small output should not background");
}

#[test]
fn TestShouldBackgroundNoCommandVeryLargeOutput() {
	// No command = can't determine if slow → passes through.
	let Result = ShouldBackground(&"x".repeat(10000), None);
	assert!(
		Result.is_none(),
		"no-command content should not be backgrounded (needs known prefix)"
	);
}

#[test]
fn TestShouldBackgroundVariousSlowPrefixes() {
	SlowCommandPrefixes.iter().take(5).for_each(|Prefix| {
		let Command = format!("{} --some-flag", Prefix);
		let Result = ShouldBackground(&"x".repeat(5000), Some(&Command));
		assert!(
			Result.is_some(),
			"prefix '{}' should trigger backgrounding for command: {}",
			Prefix,
			Command
		);
	});
}

#[test]
fn TestShouldBackgroundEmptyCommandNoContent() {
	let Result = ShouldBackground("", None);
	assert!(Result.is_none(), "empty content should never be backgrounded");
}

#[test]
fn TestShouldBackgroundBoundaryExactly2048Bytes() {
	let Content = "x".repeat(2048);
	let Result = ShouldBackground(&Content, Some("cargo build"));
	assert!(
		Result.is_some(),
		"exactly 2048 bytes of cargo build output should be backgrounded"
	);
}

#[test]
fn TestShouldBackgroundBoundary2047Bytes() {
	let Content = "x".repeat(2047);
	let Result = ShouldBackground(&Content, Some("cargo build"));
	assert!(Result.is_none(), "2047 bytes should be below the minimum threshold");
}

#[test]
fn TestShouldBackgroundNoCommandLargeOutput() {
	let Result = ShouldBackground(&"x".repeat(8192), None);
	assert!(
		Result.is_none(),
		"no-command output should not be backgrounded (needs known prefix)"
	);
}
