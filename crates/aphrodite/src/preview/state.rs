//! Process-wide preview length cap state (Issue #11 WS4).

use std::sync::atomic::{AtomicU32, Ordering};

/// Process-wide preview length cap in chars; 0 = unlimited. Set from
/// `[previews] preview_max_chars` (Issue #11 WS4): the key was declared in
/// the config structs but never read anywhere, so every preview knob was a
/// no-op. The builder now enforces it on EVERY path - proxy, hooks, and the
/// Hermes dylib all funnel through [`build_preview`].
pub(crate) static PREVIEW_MAX_CHARS:AtomicU32 = AtomicU32::new(0);

/// Configure the preview length cap (chars); `None`/0 = unlimited.
/// Returns the previous value so callers (tests, hot-reload) can restore it.
///
/// Wiring (Issue #11 WS4): the engine binary sets it right after
/// `MultiConfig::load`, the proxy `/reload` handler re-sets it on hot-reload,
/// and the Hermes bridge applies it via
/// `config_loader::Config::apply_previews` at dylib init.
pub fn set_preview_max_chars(max:Option<u32>) -> Option<u32> {
	let prev = PREVIEW_MAX_CHARS.swap(max.unwrap_or(0), Ordering::Relaxed);
	if prev == 0 { None } else { Some(prev) }
}

/// Current preview cap in chars (0 = unlimited). Test/visibility helper.
pub fn preview_max_chars() -> u32 { PREVIEW_MAX_CHARS.load(Ordering::Relaxed) }

/// Serializes tests across modules that mutate the process-global preview
/// cap (`cargo test` runs module test-bodies concurrently; the cap is
/// process-wide, so config_loader's `apply_previews` test and this module's
/// cap tests must not interleave).
#[cfg(test)]
pub(crate) fn preview_cap_test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
