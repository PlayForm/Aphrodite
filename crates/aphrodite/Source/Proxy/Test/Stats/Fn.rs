//! The `/stats` payload: every counter group, and the store values that are
//! read live rather than counted.

use crate::Configuration::Proxy::Mode::Enum::Enum as ProxyMode;
use crate::Proxy::Test::Support::State::Fn::TestState;
use crate::Proxy::Test::Support::State::Fn::TestStateWithCCR;

#[test]
fn TestStatsJSONModes() {
	let Cache = TestState();
	let Stats = Cache.StatsJSON();
	assert_eq!(Stats["mode"], "cache");
	assert_eq!(Stats["proxy"], "aphrodite");

	let mut Aph = TestState();
	Aph.Mode = ProxyMode::Token;
	let Stats = Aph.StatsJSON();
	assert_eq!(Stats["mode"], "token");
}

// ── T13 (F12): the enabled/disabled flag and the calls-stats object
// must both be visible - a duplicate JSON key used to let the stats
// object silently shadow the boolean. ──
#[test]
fn TestStatsJSONToolRelayEnabledFlagNotShadowed() {
	let mut State = TestState();
	State.ToolRelay = true;
	let Stats = State.StatsJSON();
	assert_eq!(Stats["tool_relay_enabled"], true);
	assert!(
		Stats["tool_relay"].is_object(),
		"the calls-stats object must still be present under its own key"
	);
	assert!(Stats["tool_relay"]["total"].is_u64());
}

/// Regression: `ccr_store.entries` / `bytes_approx` on `/stats` used to be
/// atomics that nothing ever incremented, so a full store still reported
/// `entries: 0` - the same "telemetry that lies" class as `FillPct`
/// being read as store occupancy. They must reflect the live backend.
#[test]
fn TestStatsJSONCCRStoreReportsLiveValues() {
	let State = TestStateWithCCR();
	let ccr = State.ccr.clone().expect("test state has a store");
	assert!(ccr.put_checked("live-entry", "payload"));

	let v = State.StatsJSON();
	assert_eq!(v["ccr_store"]["entries"], 1, "entries must be live, not a dead counter");
	// The in-memory backend has no `stats_db`, so the byte total stays 0
	// here; the SQLite backend's own stats test covers the real number.
	assert_eq!(v["ccr_store"]["bytes_approx"], 0);
}
