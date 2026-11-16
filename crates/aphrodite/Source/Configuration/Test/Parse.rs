//! Parsing the document itself - what an absent table means.

use super::Toml;

// ── Point 5 (feedback): a hook-only configuration that omits the
// `[proxies]` table entirely must parse, defaulting `Proxies` to an
// empty Vec - not fail with `missing field 'proxies'`. ──
#[test]
fn TestMultiConfigurationWithoutProxiesTableDefaultsToEmpty() {
	let Mc = Toml(
		r#"
		[compression]
		engine_threshold_pct = 45
		"#,
	);
	assert!(
		Mc.Proxies.is_empty(),
		"missing [proxies] table must default to an empty Vec, got {} entries",
		Mc.Proxies.len()
	);
}

// ── Point 5 (feedback): an explicit empty `proxies = []` also parses. ──
#[test]
fn TestMultiConfigurationExplicitEmptyProxies() {
	let Mc = Toml(
		r#"
		proxies = []
		"#,
	);
	assert!(Mc.Proxies.is_empty());
}
