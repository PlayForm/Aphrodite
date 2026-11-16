//! The explicit subcommands.

/// aphrodite subcommands.
///
/// Running the proxy is the default, so it is spelled `Run` only when written
/// out; an invocation with no subcommand parses to `None` and means the same
/// thing. The flag defaults here are the CLI's own - they are not the TOML
/// defaults, and `setup` writes the resolved values into the file it
/// generates.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum Enum {
	/// Run the proxy server (default).
	Run,
	/// Bootstrap: copy binary, create config, register with hermes, launch
	/// proxy.
	Setup {
		/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
		#[arg(
			long = "api-key",
			value_name = "API_KEY",
			env = "APHRODITE_API_KEY",
			hide_env_values = true
		)]
		Key:Option<String>,

		/// Upstream API base URL.
		#[arg(
			long = "apiurl",
			value_name = "APIURL",
			env = "APHRODITE_API_URL",
			default_value = ""
		)]
		URL:String,

		/// Model name to forward.
		#[arg(long, env = "APHRODITE_MODEL", default_value = "")]
		Model:String,

		/// Cache proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(
			long = "cache-port",
			value_name = "CACHE_PORT",
			env = "APHRODITE_CACHE_PORT",
			default_value = "9797"
		)]
		Cache:u16,

		/// Token proxy listen port. Override per-instance to run multiple
		/// concurrent Hermes Agents on the same machine.
		#[arg(
			long = "token-port",
			value_name = "TOKEN_PORT",
			env = "APHRODITE_TOKEN_PORT",
			default_value = "9798"
		)]
		Token:u16,

		/// Skip launching the proxy after setup.
		#[arg(long = "no-launch")]
		NoLaunch:bool,

		/// Force re-setup even if already installed.
		#[arg(long = "force")]
		Force:bool,
	},
}
