//! `Assess`: a configuration path as one of the four reload situations.

use super::super::Assessment::Enum::Enum as Assessment;

/// Read `Path` and decide what a reload of it means, touching no session state.
///
/// The whole filesystem read and TOML parse happen here, so the caller can take
/// the session lock afterwards and hold it for a field write and nothing else.
///
/// An empty path never reaches the filesystem: `then` defers the read, so a
/// host that passes no path costs a boolean and nothing more. A read that fails
/// is `Unreadable` - the normal search miss, kept silent on purpose - while a
/// read that succeeds and a parse that fails is `Invalid`, which is the one case
/// that reaches the log surface.
pub(crate) fn Fn(Path:&str) -> Assessment {
	let Content = match (!Path.is_empty()).then(|| std::fs::read_to_string(Path)).and_then(Result::ok) {
		Some(Content) => Content,
		None => return Assessment::Unreadable,
	};

	match Content.parse::<toml::Table>() {
		Ok(Table) => {
			Table
				.get("compression")
				.and_then(|Value| Value.as_table())
				.and_then(|Compression| Compression.get("context_engine"))
				.and_then(|Value| Value.as_bool())
				.map_or(Assessment::Omitted, |Enabled| Assessment::Ready { Enabled })
		},
		Err(Error) => {
			// Issue #38 class: a found-but-broken TOML on the reload path used
			// to be discarded silently (`.parse().ok()`), indistinguishable from
			// "config not set". Warn on a real log surface (stderr fallback when
			// no tracing subscriber exists) and record the failure on the state
			// + response so `AphroditeStats` can self-diagnose.
			crate::ConfigurationLoader::Warn::Fn::Fn(std::path::Path::new(Path), &Error, "keeping previous value");

			Assessment::Invalid { Diagnostic:format!("{Path}: {Error}") }
		},
	}
}
