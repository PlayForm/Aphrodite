//! The one credential rule the standalone path has.

use aphrodite::Configuration::CLI::Struct::Struct as CLI;

/// Refuse to start a standalone proxy with no upstream API key.
///
/// `APIKey` is optional at parse time so `setup` and keyless invocations do not
/// require it; the proxy path is the one place it is mandatory, because a proxy
/// with an empty key forwards an empty `Bearer` and every request fails upstream
/// instead of failing here with a message naming the fix.
///
/// This lives on its own rather than inside proxy assembly because it is not
/// assembly: it is a precondition, it applies to exactly one of the two
/// configuration sources, and a caller reading the listener table should not have
/// to find it in the middle of a `match`.
pub(crate) fn Fn(CLI:&CLI) -> anyhow::Result<()> {
	CLI.Key
		.trim()
		.is_empty()
		.then(|| {
			anyhow::anyhow!(
				"no API key configured - set APHRODITE_API_KEY env var or pass --api-key, or run `aphrodite setup`"
			)
		})
		.map_or(Ok(()), Err)
}
