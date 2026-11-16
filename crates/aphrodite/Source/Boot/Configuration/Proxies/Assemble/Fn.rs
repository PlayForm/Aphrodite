//! One listener per `[[proxies]]` entry.

use aphrodite::Configuration::{CLI::Struct::Struct as CLI, Proxy::Multi::Struct::Struct as MultiConfiguration};

/// Resolve every `[[proxies]]` entry, in document order.
///
/// An unnamed entry is named by its listen address: the name reaches the log
/// lines and the bind error, so a nameless listener would make a port conflict
/// impossible to attribute to an entry.
///
/// Collection is `collect` into a `Result`, so the first entry that fails
/// resolution aborts the whole table rather than starting the entries that
/// happened to resolve.
pub(crate) fn Fn(Configuration:&MultiConfiguration) -> anyhow::Result<Vec<(String, CLI)>> {
	Configuration
		.Proxies
		.iter()
		.map(|p| {
			let CLI = Configuration.Resolve(p)?;
			let Name = p.Name.clone().unwrap_or_else(|| format!("{}", CLI.Listen));

			Ok((Name, CLI))
		})
		.collect()
}
