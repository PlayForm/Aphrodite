//! Backgrounding decisions: the size floor, the prefix table and its
//! boundaries.

use crate::PollWorker::Should::{Command::Prefix::Const::Slow, Entry::Fn::After};

#[test]
fn Large() {
	let Result = After(&"x".repeat(5000), Some("cargo build --release"));
	assert!(Result.is_some(), "cargo build should be backgrounded");
}

#[test]
fn Tiny() {
	let Result = After("short output", Some("cargo build"));
	assert!(Result.is_none(), "small output should not be backgrounded");
}

#[test]
fn Unknown() {
	// Unknown command that doesn't match a slow prefix - passes through.
	let Result = After(&"x".repeat(10000), Some("some-slow-script.sh"));
	assert!(
		Result.is_none(),
		"unknown command should NOT be backgrounded (slow-prefix check)"
	);
}

#[test]
fn Small() {
	let Result = After("small", None);
	assert!(Result.is_none(), "no command + small output should not background");
}

#[test]
fn Huge() {
	// No command = can't determine if slow → passes through.
	let Result = After(&"x".repeat(10000), None);
	assert!(
		Result.is_none(),
		"no-command content should not be backgrounded (needs known prefix)"
	);
}

#[test]
fn Prefixes() {
	Slow.iter().take(5).for_each(|Prefix| {
		let Command = format!("{} --some-flag", Prefix);
		let Result = After(&"x".repeat(5000), Some(&Command));
		assert!(
			Result.is_some(),
			"prefix '{}' should trigger backgrounding for command: {}",
			Prefix,
			Command
		);
	});
}

#[test]
fn Empty() {
	let Result = After("", None);
	assert!(Result.is_none(), "empty content should never be backgrounded");
}

#[test]
fn Floor() {
	let Content = "x".repeat(2048);
	let Result = After(&Content, Some("cargo build"));
	assert!(
		Result.is_some(),
		"exactly 2048 bytes of cargo build output should be backgrounded"
	);
}

#[test]
fn Under() {
	let Content = "x".repeat(2047);
	let Result = After(&Content, Some("cargo build"));
	assert!(Result.is_none(), "2047 bytes should be below the minimum threshold");
}

#[test]
fn Bare() {
	let Result = After(&"x".repeat(8192), None);
	assert!(
		Result.is_none(),
		"no-command output should not be backgrounded (needs known prefix)"
	);
}
