//! The values `setup` acts on.

use super::super::Command::Enum::Enum as Command;

/// Arguments for the `setup` subcommand.
///
/// A plain struct, not a clap type: `setup` writes these into
/// `aphrodite.toml`, registers the plugin and launches the proxy, and none of
/// that should care how the values arrived. The `From` impl is the only place
/// the CLI shape is unwrapped, and it is also what makes a non-`Setup`
/// invocation produce the documented defaults instead of panicking.
#[derive(Debug, Clone)]
pub struct Struct {
	/// API key for the upstream LLM provider (uses APHRODITE_API_KEY env).
	pub APIKey:Option<String>,
	/// Upstream API base URL.
	pub APIURL:String,
	/// Model name to forward.
	pub Model:String,
	/// Cache proxy listen port.
	pub CachePort:u16,
	/// Token proxy listen port.
	pub TokenPort:u16,
	/// Skip launching the proxy after setup.
	pub NoLaunch:bool,
	/// Force re-setup even if already installed.
	pub Force:bool,
}

impl From<Command> for Struct {
	/// Copy the `setup` flags out, or fall back to the documented defaults.
	///
	/// The fallback arm is not dead code: `From` is total over `Command`, so
	/// every non-`Setup` invocation lands there. The ports (9797/9798) match
	/// the CLI defaults exactly - a mismatch here would silently write a
	/// different config than the flags advertise.
	fn from(Command:Command) -> Self {
		match Command {
			Command::Setup { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force } => {
				Self { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force }
			},
			_ => {
				Self {
					APIKey:None,
					APIURL:String::new(),
					Model:String::new(),
					CachePort:9797,
					TokenPort:9798,
					NoLaunch:false,
					Force:false,
				}
			},
		}
	}
}
