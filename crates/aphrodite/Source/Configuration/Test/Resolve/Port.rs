//! Listen ports, and the per-mode overrides that move them.

use super::super::{Guard::EnvironmentGuard, MultiConfigurationFromToml};

#[test]
fn TestResolveDefaultPortsPerMode() {
	let _G = EnvironmentGuard();
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
	unsafe { std::env::remove_var("APHRODITE_TOKEN_PORT") };

	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "cache"
		mode = "cache"
		api_key = "test-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.Listen.port(), 9797); // default listen, no override present

	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "token"
		mode = "token"
		api_key = "test-key"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.Listen.port(), 9797); // still 9797: no explicit `Listen` was set in the TOML
}

#[test]
fn TestResolveExplicitPortOverrideViaEnvironment() {
	let _G = EnvironmentGuard();
	unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "19797") };
	let Mc = MultiConfigurationFromToml(
		r#"
		[[proxies]]
		name = "cache"
		mode = "cache"
		api_key = "test-key"
		listen = "127.0.0.1:9797"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.Listen.port(), 19797);
	unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
}
