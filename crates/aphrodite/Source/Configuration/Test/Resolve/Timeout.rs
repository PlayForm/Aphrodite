//! The upstream timeout ceiling.

use super::super::Toml;

#[test]
fn TestResolveTimeoutClampedTo600() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "slow"
		api_key = "test-key"
		timeout = 9999
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.Timeout, 600);
}

#[test]
fn TestResolveTimeoutUnderMaximumIsUnchanged() {
	let Mc = Toml(
		r#"
		[[proxies]]
		name = "normal"
		api_key = "test-key"
		timeout = 120
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.Timeout, 120);
}
