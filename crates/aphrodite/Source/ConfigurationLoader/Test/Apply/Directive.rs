//! `[directives]` - discovery order and the empty-`Active` seeding rule.

use std::collections::HashMap;

use super::super::Guard::{CwdGuard, Guarded};
use crate::ConfigurationLoader::Configuration::Struct::Struct as Configuration;

// ── 05-T2 (P1/G3): directives load whenever a `directives/` dir exists,
// independent of whether `[directives] active` is empty - the pre-05
// gating on a non-empty `Active` left `state.directives` empty on cold
// start, making runtime discover-then-activate impossible. This test runs
// from a temp cwd containing a `directives/` dir so it is hermetic. ──
#[test]
fn TestDirectivesLoadedEvenWhenActiveEmpty() {
	let _G = CwdGuard.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();

	let Temporary = std::env::temp_dir().join(format!(
		"aphrodite-cfg-directives-{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	std::fs::create_dir_all(Temporary.join("directives")).unwrap();
	std::fs::write(Temporary.join("directives").join("focus.md"), "# focus\nstay targeted").unwrap();

	let Original = std::env::current_dir().unwrap();
	std::env::set_current_dir(&Temporary).unwrap();

	// `Active` is empty - directives must still load.
	let cfg = Configuration {
		raw:"[directives]\nactive = []\n".parse().unwrap(),
		Overrides:HashMap::new(),
		ParseFailure:None,
	};
	let mut State = crate::State::Struct::AphroditeState::default();
	Guarded(&cfg, &mut State);

	std::env::set_current_dir(&Original).unwrap();
	let _ = std::fs::remove_dir_all(&Temporary);

	assert!(
		State.Directive.Loaded.contains_key("focus"),
		"directives must load even when [directives] active is empty"
	);
	// With empty TOML `Active` and directives loaded from disk, the
	// fallback seeds whatever of [focus, foresight, lazy] exists in the
	// loaded set - this temp dir only has focus.md, so focus is seeded.
	assert!(
		!State.Directive.Active.is_empty(),
		"empty active list should seed focus + foresight defaults"
	);
	assert!(State.Directive.Active.iter().any(|name| name == "focus"));
}

// ── Issue #6: $APHRODITE_DIRECTIVES_DIR is the FIRST candidate and
// must win over a cwd `directives/` dir. Shares CWD_GUARD with the other
// cwd-mutating test since both set_current_dir. ──
#[test]
fn TestDirectivesEnvironmentOverrideWinsOverCwd() {
	let _G = CwdGuard.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();

	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let EnvironmentDirectory = std::env::temp_dir().join(format!("aphrodite-cfg-envdir-{Stamp}"));
	let CwdDirectory = std::env::temp_dir().join(format!("aphrodite-cfg-env-cwd-{Stamp}"));
	std::fs::create_dir_all(&EnvironmentDirectory).unwrap();
	std::fs::create_dir_all(CwdDirectory.join("directives")).unwrap();
	std::fs::write(EnvironmentDirectory.join("envwin.md"), "# envwin\nfrom env override").unwrap();
	std::fs::write(CwdDirectory.join("directives").join("cwdwin.md"), "# cwdwin\nfrom cwd").unwrap();

	let Original = std::env::current_dir().unwrap();
	std::env::set_current_dir(&CwdDirectory).unwrap();
	unsafe { std::env::set_var("APHRODITE_DIRECTIVES_DIR", &EnvironmentDirectory) };

	let mut State = crate::State::Struct::AphroditeState::default();
	Guarded(&Configuration::default(), &mut State);

	unsafe { std::env::remove_var("APHRODITE_DIRECTIVES_DIR") };
	std::env::set_current_dir(&Original).unwrap();
	let _ = std::fs::remove_dir_all(&EnvironmentDirectory);
	let _ = std::fs::remove_dir_all(&CwdDirectory);

	assert!(
		State.Directive.Loaded.contains_key("envwin"),
		"$APHRODITE_DIRECTIVES_DIR must win as the first candidate"
	);
	assert!(
		!State.Directive.Loaded.contains_key("cwdwin"),
		"the cwd directives/ candidate must not beat the env override"
	);
}
