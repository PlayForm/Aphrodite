//! Resolving one `[[proxies]]` entry into the CLI-shaped configuration the
//! proxy consumes.

use std::net::SocketAddr;

use crate::Configuration::{
	CLI::Struct::Struct as CLI,
	Environment::{Text::Fn::Fn as Text, Warn::Fn::Fn as Warn},
	Proxy::Mode::Enum::Enum as ProxyMode,
};
use super::super::Struct::Struct;

impl Struct {
	/// Resolve one `[[proxies]]` entry into a `CLI`, defaults applied.
	///
	/// API key fallback chain: `proxy.api_key` → `defaults.api_key` →
	/// `APHRODITE_API_KEY` (no provider-specific key names are probed).
	/// Returns an error if no API key is found after all fallbacks.
	///
	/// The rest of the chain is env > TOML (proxy over defaults) > hardcoded
	/// default, per knob. Two validation failures are hard errors rather than
	/// fallbacks - a missing API key and `max_output >= max_context` - because
	/// both produce a proxy that accepts traffic and then cannot serve it.
	/// Everything else degrades with a warning: an unparseable listen address
	/// is the one exception that errors, since a proxy that listens nowhere is
	/// indistinguishable from a crash.
	///
	/// `None` on any knob means "not configured here" and never "zero": each
	/// chain ends in the compiled-in default named beside it.
	pub fn Resolve(&self, Entry:&crate::Configuration::Proxy::Proxy::Struct::Struct) -> anyhow::Result<CLI> {
		let Defaults = self.Defaults.as_ref();
		let Key:String = Entry
			// API key fallback chain: explicit config → APHRODITE_API_KEY
			// (no provider-specific key names are probed - the upstream is
			// whatever APHRODITE_API_URL points at, keyed by APHRODITE_API_KEY)
			.Key
			.clone()
			.or_else(|| Defaults.and_then(|Defaults| Defaults.Key.clone()))
			.or_else(|| std::env::var("APHRODITE_API_KEY").ok())
			.unwrap_or_default();
		Key.is_empty()
			.then(|| {
				anyhow::anyhow!("no API key configured - set APHRODITE_API_KEY env var or api_key in aphrodite.toml")
			})
			.map_or(Ok(()), Err)?;
		// Resolve listen: must parse or fail (no silent default when listen is
		// explicitly set).  After parsing, override with env var if set - this
		// lets multiple concurrent Hermes Agent instances each point at their
		// own proxy pair without editing aphrodite.toml.
		//
		// The default is built, not parsed: `"127.0.0.1:9797".parse().unwrap()` is an
		// `expect` on a constant, which cannot fail but reads as if it could.
		let Listen:SocketAddr = match Entry.Listen.as_deref() {
			Some(s) => s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?,
			None => SocketAddr::from(([127, 0, 0, 1], 9797)),
		};
		let Listen = match (Entry.Mode.as_deref(), Entry.Name.as_deref()) {
			(_, Some("cache")) | (Some("cache"), _) => Self::Override(Listen, "APHRODITE_CACHE_PORT"),
			(_, Some("token")) | (Some("token"), _) => Self::Override(Listen, "APHRODITE_TOKEN_PORT"),
			_ => Listen,
		};
		// Validate max_output < max_context
		let Context = Entry.Context.unwrap_or(1_000_000);
		let Output = Entry.Output.unwrap_or(384_000);
		(Output >= Context)
			.then(|| anyhow::anyhow!("max_output ({Output}) must be less than max_context ({Context})"))
			.map_or(Ok(()), Err)?;
		Ok(CLI {
			Command:None,
			Mode:match Entry.Mode.as_deref() {
				Some("token") => ProxyMode::Token,
				Some("cache") => ProxyMode::Cache,
				None => {
					tracing::info!("no mode specified, defaulting to token");
					ProxyMode::Token
				},
				Some(Other) => {
					tracing::warn!("unknown mode {:?}, defaulting to token", Other);
					ProxyMode::Token
				},
			},
			Listen,
			// env > TOML (proxy > defaults) > hardcoded default (report 07
			// F1/T17) - previously only the API-key chain and the two port
			// vars pierced the TOML in this path, contradicting every shipped
			// TOML's own header comment ("Env vars override TOML values at
			// runtime"). `Mode`/`Listen` are deliberately NOT given a
			// blanket env override here: a single process-wide
			// `APHRODITE_MODE`/`APHRODITE_LISTEN` would incorrectly apply to
			// every `[[proxies]]` entry at once, breaking the cache/token
			// dual-proxy split that the existing per-mode
			// `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT` overrides above
			// are already careful to respect.
			URL:Text("APHRODITE_API_URL")
				.or_else(|| Entry.URL.clone())
				.or_else(|| Defaults.and_then(|Defaults| Defaults.URL.clone()))
				.unwrap_or_else(|| "https://api.openai.com".into()),
			Key,
			Model:Text("APHRODITE_MODEL")
				.or_else(|| Entry.Model.clone())
				.or_else(|| Defaults.and_then(|Defaults| Defaults.Model.clone()))
				.unwrap_or_else(|| "default-model".into()),
			Context,
			Output,
			// Resolve from toml - proxy.rs handles None default
			Database:Text("APHRODITE_DB")
				.or_else(|| Entry.Database.clone())
				.filter(|Value| !Value.is_empty())
				.map(Into::into),
			TTL:Warn::<u64>("APHRODITE_CCR_TTL")
				.or(Entry.TTL)
				.or_else(|| Defaults.and_then(|Defaults| Defaults.TTL))
				.unwrap_or(3600),
			// Retention caps: env > [compression] > compiled-in default.
			// 0 is passed through (not normalised here) - the store
			// treats 0 as unlimited, and keeping the operator's literal
			// value in `/stats/db` is more honest than silently rewriting
			// it to null.
			Entries:Warn::<usize>("APHRODITE_STORE_MAX_ENTRIES")
				.or_else(|| self.Compression.as_ref().and_then(|Compression| Compression.Entries))
				.unwrap_or(100_000),
			Bytes:Warn::<u64>("APHRODITE_STORE_MAX_BYTES")
				.or_else(|| self.Compression.as_ref().and_then(|Compression| Compression.Bytes))
				.unwrap_or(536_870_912),
			NoMarker:false,
			Relay:Entry.Relay.unwrap_or(false),
			Callback:Text("APHRODITE_NOTIFY_URL").or_else(|| Entry.Callback.clone()),
			Auth:Text("APHRODITE_NOTIFY_KEY").or_else(|| Entry.Auth.clone()),
			Dev:Entry.Dev.unwrap_or(false),
			LogCompact:false,
			Timeout:{
				let Configured = Entry.Timeout.unwrap_or(300);
				let Clamped = Configured.min(600);

				// The clamp is the gate: only a value it changed is worth reporting.
				(Clamped != Configured)
					.then_some(Configured)
					.into_iter()
					.for_each(|Excess| tracing::warn!("timeout {}s exceeds maximum 600s, clamping", Excess));

				Clamped
			},
		})
	}
}
