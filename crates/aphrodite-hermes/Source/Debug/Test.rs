//! The logging pass: one subscriber, one recorded ownership, one emission
//! decision, and a warning that lands either way.
//!
//! None of these probes touches the process environment. The one that used to
//! (`unsafe { std::env::set_var("APHRODITE_DEBUG_HOME", ...) }` to point the core
//! sink at a scratch directory) was unsafe for a reason a test mutex cannot fix:
//! `setenv` races `getenv` in every OTHER thread of the harness, not just in the
//! tests that hold the same lock. The bridge's decision is now a function of a
//! supplied sink, so the destination never has to be redirected at all.

use crate::Debug::{
	Flag::{Read, State},
	Init::{Fn, Ownership},
	Log::Bridge::At,
	Warn,
};

/// A scratch flag file with a unique name, so the mtime cache cannot confuse one
/// probe with another.
fn Scratch(Tag:&str, Text:&str) -> std::path::PathBuf {
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Path = std::env::temp_dir().join(format!("aphrodite-flag-{Tag}-{Stamp}"));
	std::fs::write(&Path, Text).unwrap();

	Path
}

// ── The install is idempotent by contract, so calling it repeatedly is the
// assertion: a second init must be a no-op, never a panic, never a second
// subscriber, and it must return the SAME recorded decision rather than make a
// new one. `try_init` is what makes that true when a host got there first. ──
#[test]
fn probe_subscriber_install_is_idempotent() {
	Fn::Fn();
	let First = Fn::Fn();

	assert!(
		Ownership::Fn::Fn().is_some(),
		"the install must record who owns the dispatcher, not merely perform it"
	);
	assert!(
		std::ptr::eq(First, Fn::Fn()),
		"a second install must return the recorded decision, not compute a new one"
	);
}

// ── The always-visible warn has to survive both dispatcher states: before this
// crate has installed anything (nothing is known to be listening, so the stderr
// fallback fires) and after it owns the dispatcher (the fmt layer carries it).
// Neither may panic. ──
#[test]
fn probe_warn_survives_before_and_after_the_install() {
	Warn::Fn::Fn("logging-pass probe: before any install");
	Fn::Fn();
	Warn::Fn::Fn("logging-pass probe: after the install");
}

// ── The bridge's decision, asserted at the seam that makes it a decision: an
// enabled level reaches the sink exactly once, with the effective level, the
// event name and the caller's fields in the caller's order. ──
#[test]
fn probe_bridge_hands_the_record_to_its_sink() {
	let mut State = crate::Configuration::Template::Fn::Fn().clone();
	// A non-zero level wins over the flag file, so this probe reads no
	// filesystem and depends on no session.
	State.DebugLevel = 1;

	let Captured = std::cell::RefCell::new(Vec::new());

	At::Fn::Fn(
		&State,
		"probe-root",
		"bridge_probe",
		&[("k", "v".to_string())],
		|Level, Event, Fields| {
			Captured.borrow_mut().push((
				Level,
				Event.to_string(),
				Fields
					.iter()
					.map(|(Key, Value)| ((*Key).to_string(), Value.clone()))
					.collect::<Vec<_>>(),
			));
		},
	);

	let Captured = Captured.into_inner();
	assert_eq!(Captured.len(), 1, "an enabled level must reach the sink exactly once");
	assert_eq!(Captured[0].0, 1, "the sink must receive the effective level");
	assert_eq!(Captured[0].1, "bridge_probe", "the sink must receive the event name");
	assert_eq!(
		Captured[0].2,
		vec![("k".to_string(), "v".to_string())],
		"the sink must receive the caller's fields, in the caller's order"
	);
}

// ── The quiet path: level 0 AND no flag file for the session means the sink is
// never called, so the cost of a disabled debug channel is one level read. ──
#[test]
fn probe_bridge_stays_silent_at_level_zero() {
	let mut State = crate::Configuration::Template::Fn::Fn().clone();
	State.DebugLevel = 0;

	let mut Called = false;
	At::Fn::Fn(&State, "probe-quiet-no-flag-file", "quiet", &[], |_, _, _| Called = true);

	assert!(!Called, "level 0 with no flag file for the session must not reach the sink");
}

// ── The flag reader has to tolerate the logger: the core debug logger appends
// to the same `debug.<root>` file, and a legacy toggle writes its value with no
// trailing newline, so the first log line can be fused onto the flag value. ──
#[test]
fn probe_flag_read_tolerates_a_fused_log_line() {
	let Path = Scratch("fused", "on[2026-01-01T00:00:00.000000Z] [aphrodite-debug] lvl=1 event=x\n");

	assert!(
		matches!(Read::Fn::Fn(&Path), State::Enum::Enum::On),
		"a flag fused with the first log line must still read as on"
	);

	let _ = std::fs::remove_file(&Path);
}

// ── A file that exists and says off is OFF; a file that does not exist is
// ABSENT. The two are different answers, which is why the reader stopped
// returning a `bool`. ──
#[test]
fn probe_flag_read_distinguishes_off_from_absent() {
	let Off = Scratch("off", "off\n");

	assert!(
		matches!(Read::Fn::Fn(&Off), State::Enum::Enum::Off),
		"an explicit off must read as off"
	);

	let _ = std::fs::remove_file(&Off);

	let Absent = std::env::temp_dir().join("aphrodite-flag-absent-probe");
	let _ = std::fs::remove_file(&Absent);

	assert!(
		matches!(Read::Fn::Fn(&Absent), State::Enum::Enum::Absent),
		"a missing flag file must read as absent, never as an explicit off"
	);
}
