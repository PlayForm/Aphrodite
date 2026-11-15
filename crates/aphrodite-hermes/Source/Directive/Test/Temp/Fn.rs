//! A per-test scratch home under the OS temp directory.
//!
//! Provisioning writes real files, so every test that exercises it needs a
//! directory nobody else owns. The name carries the process id and a nanosecond
//! stamp because the harness runs tests in parallel and a fixed name would let
//! two tests materialize into the same directory.

use std::path::PathBuf;

/// Create and return a fresh scratch home tagged with `tag`.
///
/// The directory is created eagerly so a test can write into it immediately;
/// cleanup is the caller's job (`remove_dir_all` at the end), which keeps a
/// failing test's artifacts on disk for inspection.
pub(crate) fn Fn(tag:&str) -> PathBuf {
	let base = std::env::temp_dir().join(format!(
		"aphrodite-directives-{tag}-{}-{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_default()
			.as_nanos(),
	));
	std::fs::create_dir_all(&base).unwrap();
	base
}
