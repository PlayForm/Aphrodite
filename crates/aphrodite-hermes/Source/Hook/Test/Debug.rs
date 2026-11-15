//! The dev-only debug logger: structured lines AND first-line flag semantics
//! must coexist in the same file.

// ── DEV-ONLY debug logger (temporary instrumentation): the env-gated
// logger must write timestamped structured lines to debug.<root> under
// the runtime home, and a toggle written to that same file must stay
// readable (first-line flag semantics coexist with the appended log
// history). Self-cleaning: the probe file is removed in the test.
#[test]
fn test_debug_file_write_smoke() {
	let _g = crate::Session::Guard::Fn::test_guard();
	// Pin BOTH the core logger and the flag reader to the same scratch
	// home (the core crate compiles without cfg(test) as a dependency,
	// so isolation relies on the dev-only $APHRODITE_DEBUG_HOME
	// override, not on cfg(test)).
	const HOME_KEY:&str = "APHRODITE_DEBUG_HOME";
	let scratch = std::env::temp_dir().join("aphrodite-debug-smoke-home");
	let _ = std::fs::create_dir_all(&scratch);
	let backup = std::env::var(HOME_KEY).ok();
	unsafe { std::env::set_var(HOME_KEY, &scratch) };
	let root = "debug-smoke";
	let path = aphrodite::Debug::Path::ForRoot::Fn::Fn(root);
	aphrodite::Debug::Level::Set::Fn::Fn(1);
	aphrodite::Debug::Root::Note::Fn::Fn(root);
	let _ = std::fs::remove_file(&path);
	aphrodite::Debug::Log::At::Fn::Fn(1, "smoke", &[("k", "v".to_string())]);
	let content = std::fs::read_to_string(&path).unwrap_or_default();
	assert!(
		content.contains("event=smoke") && content.contains("k=v"),
		"debug file must carry the structured line, got: {content:?}"
	);
	assert!(content.starts_with('['), "line must be timestamped: {content:?}");
	// Toggle written to the same file (whole-file overwrite) must still
	// resolve as a flag: first-line/prefix semantics.
	std::fs::write(&path, "on\n").unwrap();
	assert!(crate::Debug::Flag::Resolve::Fn::Fn(root), "toggle must stay readable on line 1");
	// Regression: a legacy flag file may have NO trailing newline
	// ("on" bare), so a subsequent logger append merges into line 1
	// ("on[2026-...") - the flag must still read ON (prefix match).
	std::fs::write(&path, "on").unwrap();
	aphrodite::Debug::Log::At::Fn::Fn(1, "merged", &[]);
	assert!(
		crate::Debug::Flag::Resolve::Fn::Fn(root),
		"flag must survive an appended line merged into line 1"
	);
	let _ = std::fs::remove_file(&path);
	aphrodite::Debug::Level::Set::Fn::Fn(0);
	unsafe { std::env::remove_var(HOME_KEY) };
	match backup {
		Some(v) => {
			unsafe { std::env::set_var(HOME_KEY, v) };
		},
		None => {},
	}
}
