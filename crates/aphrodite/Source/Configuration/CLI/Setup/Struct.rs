//! The values `setup` acts on.

/// Arguments for the `setup` subcommand.
///
/// A plain struct, not a clap type: `setup` writes these into `aphrodite.toml`, registers
/// the plugin and launches the proxy, and none of that should care how the values
/// arrived. `Request::Setup` is the one place the CLI shape is unwrapped, and it answers
/// `None` for an invocation that did not ask for `setup`.
///
/// There is deliberately no `From<Command>` impl. There used to be one, total over
/// `Command` so a `run` invocation would produce "the documented defaults" - which meant
/// this type could be built with a full argument set, two proxy ports and an empty
/// upstream included, for a command that has no setup arguments at all. Absence is
/// expressed by not having a `Struct`: the request enum carries no `Setup` variant for a
/// `run`.
#[derive(Debug, Clone)]
pub struct Struct {
	/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
	pub Key:Option<String>,
	/// Upstream API base URL.
	pub URL:String,
	/// Model name to forward.
	pub Model:String,
	/// Cache proxy listen port.
	pub Cache:u16,
	/// Token proxy listen port.
	pub Token:u16,
	/// Skip launching the proxy after setup.
	pub NoLaunch:bool,
	/// Force re-setup even if already installed.
	pub Force:bool,
}
