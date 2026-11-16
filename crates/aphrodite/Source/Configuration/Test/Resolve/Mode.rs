//! `Mode` - the one knob with a documented fallback instead of an error.

use crate::Configuration::{Proxy::Mode::Enum::Enum as ProxyMode, Test::Toml};

#[test]
fn TestResolveInvalidModeFallsBackToToken() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "weird"
		mode = "not_a_real_mode"
		api_key = "test-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert!(matches!(CLI.Mode, ProxyMode::Token));
}

#[test]
fn TestResolveMissingModeFallsBackToToken() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "no_mode"
		api_key = "test-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert!(matches!(CLI.Mode, ProxyMode::Token));
}
