//! Resolving one `[[proxies]]` entry into the CLI-shaped configuration the
//! proxy consumes.

use std::net::SocketAddr;

use crate::Configuration::{
	CLI::Struct::Struct as CLI,
	Environment::{Text::Fn::EnvironmentText, Warn::Fn::EnvironmentParseWarn},
	Proxy::Mode::Enum::Enum as ProxyMode,
};
use super::super::Struct::Struct;

impl Struct {
	/// Resolve a ProxyConfig with defaults applied.
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
	pub fn Resolve(&self, cfg:&crate::Configuration::Proxy::Proxy::Struct::Struct) -> anyhow::Result<CLI> {
		let d = self.Defaults.as_ref();
		let APIKey:String = cfg
			// API key fallback chain: explicit config → APHRODITE_API_KEY
			// (no provider-specific key names are probed - the upstream is
			// whatever APHRODITE_API_URL points at, keyed by APHRODITE_API_KEY)
			.APIKey
			.clone()
			.or_else(|| d.and_then(|d| d.APIKey.clone()))
			.or_else(|| std::env::var("APHRODITE_API_KEY").ok())
			.unwrap_or_default();
		APIKey
			.is_empty()
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
		let Listen:SocketAddr = match cfg.Listen.as_deref() {
			Some(s) => s.parse().map_err(|_| anyhow::anyhow!("invalid listen address: {s}"))?,
			None => SocketAddr::from(([127, 0, 0, 1], 9797)),
		};
		let Listen = match (cfg.Mode.as_deref(), cfg.name.as_deref()) {
			(_, Some("cache")) | (Some("cache"), _) => Self::ApplyPortOverride(Listen, "APHRODITE_CACHE_PORT"),
			(_, Some("token")) | (Some("token"), _) => Self::ApplyPortOverride(Listen, "APHRODITE_TOKEN_PORT"),
			_ => Listen,
		};
		// Validate max_output < max_context
		let MaximumContext = cfg.MaximumContext.unwrap_or(1_000_000);
		let MaximumOutput = cfg.MaximumOutput.unwrap_or(384_000);
		(MaximumOutput >= MaximumContext)
			.then(|| anyhow::anyhow!("max_output ({MaximumOutput}) must be less than max_context ({MaximumContext})"))
			.map_or(Ok(()), Err)?;
		Ok(CLI {
			command:None,
			Mode:match cfg.Mode.as_deref() {
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
			APIURL:EnvironmentText("APHRODITE_API_URL")
				.or_else(|| cfg.APIURL.clone())
				.or_else(|| d.and_then(|d| d.APIURL.clone()))
				.unwrap_or_else(|| "https://api.openai.com".into()),
			APIKey,
			Model:EnvironmentText("APHRODITE_MODEL")
				.or_else(|| cfg.Model.clone())
				.or_else(|| d.and_then(|d| d.Model.clone()))
				.unwrap_or_else(|| "default-model".into()),
			MaximumContext,
			MaximumOutput,
			// Resolve from toml - proxy.rs handles None default
			CCRDatabasePath:EnvironmentText("APHRODITE_DB")
				.or_else(|| cfg.CCRDatabasePath.clone())
				.filter(|s| !s.is_empty())
				.map(Into::into),
			CCRTtlSeconds:EnvironmentParseWarn::<u64>("APHRODITE_CCR_TTL")
				.or(cfg.CCRTtlSeconds)
				.or_else(|| d.and_then(|d| d.CCRTtlSeconds))
				.unwrap_or(3600),
			// Retention caps: env > [compression] > compiled-in default.
			// 0 is passed through (not normalised here) - the store
			// treats 0 as unlimited, and keeping the operator's literal
			// value in `/stats/db` is more honest than silently rewriting
			// it to null.
			StoreMaximumEntries:EnvironmentParseWarn::<usize>("APHRODITE_STORE_MAX_ENTRIES")
				.or_else(|| self.Compression.as_ref().and_then(|c| c.StoreMaximumEntries))
				.unwrap_or(100_000),
			StoreMaximumBytes:EnvironmentParseWarn::<u64>("APHRODITE_STORE_MAX_BYTES")
				.or_else(|| self.Compression.as_ref().and_then(|c| c.StoreMaximumBytes))
				.unwrap_or(536_870_912),
			NoCCRMarker:false,
			ToolRelay:cfg.ToolRelay.unwrap_or(false),
			NotifyURL:EnvironmentText("APHRODITE_NOTIFY_URL").or_else(|| cfg.NotifyURL.clone()),
			NotifyKey:EnvironmentText("APHRODITE_NOTIFY_KEY").or_else(|| cfg.NotifyKey.clone()),
			Dev:cfg.Dev.unwrap_or(false),
			LogCompact:false,
			Timeout:{
				let Configured = cfg.Timeout.unwrap_or(300);
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
