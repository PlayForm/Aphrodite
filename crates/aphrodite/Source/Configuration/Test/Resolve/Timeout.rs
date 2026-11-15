//! The upstream timeout ceiling.

use super::super::MultiConfigurationFromToml;

#[test]
fn TestResolveTimeoutClampedTo600() {
	let Mc = MultiConfigurationFromToml(
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
	let Mc = MultiConfigurationFromToml(
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
