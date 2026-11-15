//! Configure the preview length cap.

use std::sync::atomic::Ordering;

use super::Maximum::PreviewMaximumChars;

/// Set the preview length cap in characters; `None` or 0 means unlimited.
///
/// Returns the previous value so callers - tests and the hot-reload path - can
/// restore it. Wiring: the engine binary sets it right after `MultiConfig::load`,
/// the proxy `/reload` handler re-sets it on hot reload, and the Hermes bridge
/// applies it via `ConfigurationLoader::Configuration::Struct::Struct::apply_previews` at dylib init.
///
/// It is not a validation point: a cap smaller than a marker is accepted and simply
/// truncates every preview, because refusing the value would leave the previous cap
/// in force with no signal that configuration was ignored.
pub fn Fn(Maximum:Option<u32>) -> Option<u32> {
	let Previous = PreviewMaximumChars.swap(Maximum.unwrap_or(0), Ordering::Relaxed);

	match Previous == 0 {
		true => None,
		false => Some(Previous),
	}
}
