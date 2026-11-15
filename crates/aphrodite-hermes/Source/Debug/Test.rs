//! The logging pass: one subscriber, one emission path, one always-visible warn.

// The install is idempotent by contract, so calling it three times is the
// assertion: a second init must be a no-op, never a panic and never a second
// subscriber. `try_init` is what makes that true when a host got there first.
#[test]
fn test_subscriber_install_is_idempotent() {
	crate::Debug::Init::Fn::Fn();
	crate::Debug::Init::Fn::Fn();
	crate::Debug::Init::Fn::Fn();

	assert!(
		tracing::dispatcher::has_been_set(),
		"the dylib must own a dispatcher after its own install in a bare process"
	);
}

// The always-visible warn has to survive BOTH dispatcher states: one where the
// dylib installed its subscriber and one where it did not. Neither may panic,
// and neither may print twice (the stderr fallback is gated on
// `has_been_set`).
#[test]
fn test_warn_survives_either_dispatcher_state() {
	crate::Debug::Warn::Fn::Fn("logging-pass probe: no subscriber was installed here");
	crate::Debug::Init::Fn::Fn();
	crate::Debug::Warn::Fn::Fn("logging-pass probe: this crate installed the subscriber");
}

// The per-session debug file is the artifact the flag toggle is read from, and
// it is written by the bridge - NOT by a subscriber layer, precisely so it
// keeps working when the install above loses to a host. This proves the
// bridge still produces it, and that the file keeps its first-line flag
// semantics after a bridge event lands in it.
#[test]
fn test_bridge_event_still_writes_the_session_file() {
	let _g = crate::Session::Guard::Fn::test_guard();

	const HOME_KEY:&str = "APHRODITE_DEBUG_HOME";
	let scratch = std::env::temp_dir().join("aphrodite-bridge-log-home");
	let _ = std::fs::create_dir_all(&scratch);
	let backup = std::env::var(HOME_KEY).ok();
	unsafe { std::env::set_var(HOME_KEY, &scratch) };

	let root = "bridge-log-probe";
	let path = aphrodite::Debug::Path::ForRoot::Fn::Fn(root);
	let _ = std::fs::remove_file(&path);

	// Flag file ON, then one bridge event through the tracing emission point.
	std::fs::write(&path, "on\n").unwrap();
	// The core sink targets `debug.<ROOT>` from the process-global root that
	// `pre_llm_call` publishes (`Debug::Session::Record`), so the test publishes
	// it exactly as a real turn would - without this the sink has no path and
	// degrades to silence by design.
	aphrodite::Debug::Root::Note::Fn::Fn(root);
	let state = crate::Configuration::Template::Fn::config_template();
	crate::Debug::Log::Bridge::Fn::Fn(state, root, "bridge_probe", &[("k", "v".to_string())]);

	let content = std::fs::read_to_string(&path).unwrap_or_default();
	assert!(
		content.contains("event=bridge_probe") && content.contains("k=v"),
		"the bridge must still append its fields to debug.<root>, got: {content:?}"
	);
	assert!(
		crate::Debug::Flag::Resolve::Fn::Fn(root),
		"the toggle must stay readable after a bridge event lands in the same file"
	);

	let _ = std::fs::remove_file(&path);
	match backup {
		Some(v) => {
			unsafe { std::env::set_var(HOME_KEY, v) };
		},
		None => {
			unsafe { std::env::remove_var(HOME_KEY) };
		},
	}
}
