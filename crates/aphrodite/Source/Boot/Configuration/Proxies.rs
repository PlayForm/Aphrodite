//! Turning the config (or the CLI) into the listener table.
//!
//! One listener per `[[proxies]]` entry, or a single CLI-invoked proxy when there
//! is no config file. The shared `[compression]` table is not assembled here -
//! `Configuration::compression` reads it from the same document, because a
//! listener's thresholds are a property of the whole process, not of one entry.

use aphrodite::Configuration::{CLI::Struct::Struct as CLI, Proxy::Multi::Struct::Struct as MultiConfiguration};

/// Build the listener table from the config, or from the CLI when there is none.
///
/// Returns `(name, cli)` per listener. Fails on a `[[proxies]]` entry that cannot
/// resolve, and - for the standalone path - on a missing API key, so a proxy never
/// starts forwarding an empty `Bearer`.
pub fn Assemble(
	MultiConfiguration:Option<MultiConfiguration>,
	CLIFallback:Option<CLI>,
) -> anyhow::Result<Vec<(String, CLI)>> {
	let Proxies:Vec<(String, CLI)> = match MultiConfiguration {
		Some(Configuration) => {
			Configuration
				.Proxies
				.iter()
				.map(|p| {
					let CLI = Configuration.Resolve(p)?;
					let name = p.name.clone().unwrap_or_else(|| format!("{}", CLI.Listen));
					Ok((name, CLI))
				})
				.collect::<anyhow::Result<Vec<_>>>()?
		},
		None => {
			let CLI = CLIFallback.expect("cli_fallback set when use_multi_config is false");
			// A standalone (no aphrodite.toml) proxy launch needs an upstream key.
			// `APIKey` is optional at parse time so `setup`/keyless invocations
			// don't require it; enforce it here for the proxy path specifically,
			// with a clear error instead of silently forwarding an empty Bearer.
			match CLI.APIKey.trim().is_empty() {
				true => {
					anyhow::bail!(
						"no API key configured - set APHRODITE_API_KEY env var or pass --api-key, or run `aphrodite \
						 setup`"
					);
				},
				false => {},
			}
			let name = format!("{}", CLI.Listen);
			vec![(name, CLI)]
		},
	};

	Ok(Proxies)
}
