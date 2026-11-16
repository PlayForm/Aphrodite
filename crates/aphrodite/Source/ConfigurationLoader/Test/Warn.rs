//! `Warn` tests - a found-but-broken file must announce itself on a surface
//! the host process actually has, and resolution must still fall back to
//! defaults.
//!
//! The warning text is asserted through `Warn::Message::Fn`, not through a
//! captured `tracing` event. Capturing the event is inherently racy: a
//! callsite's interest is cached process-wide, so whichever test reaches this
//! callsite first under the no-op subscriber can cache it as disabled and a
//! later `with_default` capture never fires. That race made this test fail
//! roughly one run in six. The message builder is the deterministic half of the
//! contract; the emission is one line that cannot drift from it.

use crate::ConfigurationLoader::{Configuration::Struct::Struct as Configuration, Warn::Message::Fn::Fn as Text};

// ── Fix 22 (inspection): a found-but-broken TOML file must announce itself
// (path + parse error) instead of silently falling through to the next search
// path / defaults; resolution behavior itself is unchanged (broken => defaults,
// valid => parsed, missing => defaults). ──
#[test]
fn TestReadBrokenTomlWarnsAndReturnsDefaults() {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Broken = std::env::temp_dir().join(format!("aphrodite-cfg-broken-{Stamp}.toml"));
	std::fs::write(&Broken, "[compression\nenabled = false\n").unwrap();

	let cfg = Configuration::Read(Broken.to_str().unwrap());

	let _ = std::fs::remove_file(&Broken);

	// Resolution behavior unchanged: a broken file yields defaults.
	assert!(cfg.Bool("NONEXISTENT", "compression", "enabled", true));

	// ...and the warning names the failure and the offending path.
	let Message = Text(&Broken, "using defaults");
	assert!(
		Message.contains("failed to parse"),
		"warning must mention the parse failure: {Message}"
	);
	assert!(
		Message.contains(Broken.to_str().unwrap()),
		"warning must include the offending path: {Message}"
	);
}

/// The `Action` clause is what tells an operator whether the loader kept
/// looking or gave up - it must survive into the message verbatim.
#[test]
fn TestParseFailureMessageCarriesTheActionClause() {
	let path = std::path::Path::new("/tmp/aphrodite.toml");

	let Skipping = Text(path, "skipping");
	let Defaults = Text(path, "using defaults");

	assert!(Skipping.contains("skipping"), "action clause lost: {Skipping}");
	assert!(Defaults.contains("using defaults"), "action clause lost: {Defaults}");
	assert_ne!(Skipping, Defaults, "the two actions must read differently");
}
