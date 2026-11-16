//! The three configurations that must be rejected rather than degraded.

use super::super::{Guard::EnvironmentGuard, Toml};

#[test]
fn TestResolveMissingAPIKeyErrors() {
	let _G = EnvironmentGuard();
	unsafe { std::env::remove_var("APHRODITE_API_KEY") };
	unsafe { std::env::remove_var("DEEPSEEK_API_KEY") };
	unsafe { std::env::remove_var("HEADROOM_DEEPSEEK_KEY") };
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "no_key"
		"#,
	);
	let Result = Mc.Resolve(&Mc.Proxies[0]);
	assert!(Result.is_err());
}

#[test]
fn TestResolveInvalidListenAddressErrors() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "bad_listen"
		api_key = "test-key"
		listen = "not-an-address"
		"#,
	);
	let Result = Mc.Resolve(&Mc.Proxies[0]);
	assert!(Result.is_err());
}

#[test]
fn TestResolveMaximumOutputMustBeLessThanMaximumContext() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "bad_budget"
		api_key = "test-key"
		max_context = 100
		max_output = 200
		"#,
	);
	let Result = Mc.Resolve(&Mc.Proxies[0]);
	assert!(Result.is_err());
}
