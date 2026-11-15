//! `[defaults]` filling in what a `[[proxies]]` entry omits.

use super::super::{Guard::EnvironmentGuard, MultiConfigurationFromToml};

#[test]
fn TestResolveDefaultsFillInMissingProxyFields() {
	// api_url/model are env-overridable since T17 - guard against the
	// process-global env vars racing with other tests in this module.
	let _G = EnvironmentGuard();
	unsafe { std::env::remove_var("APHRODITE_API_URL") };
	unsafe { std::env::remove_var("APHRODITE_MODEL") };
	let Mc = MultiConfigurationFromToml(
		r#"
		[defaults]
		api_key = "default-key"
		api_url = "https://default.example.com"
		model = "default-model-name"

		[[proxies]]
		name = "uses_defaults"
		"#,
	);
	let CLI = Mc.Resolve(&Mc.Proxies[0]).unwrap();
	assert_eq!(CLI.APIKey, "default-key");
	assert_eq!(CLI.APIURL, "https://default.example.com");
	assert_eq!(CLI.Model, "default-model-name");
}
