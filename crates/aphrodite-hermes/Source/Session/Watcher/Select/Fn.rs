//! Which events wake the config-reload loop.

/// Is this event a change to a config file?
///
/// The filter is the same one the engine's watcher uses: a `Modify` event whose
/// path mentions `aphrodite.toml`. Anything else on those directories - the
/// runtime home holds flags, logs and the hot-reload drop directory - is noise
/// the loop must not wake for.
pub fn Fn(Event:&notify::Event) -> bool {
	matches!(Event.kind, notify::EventKind::Modify(_))
		&& Event.paths.iter().any(|Path| Path.to_string_lossy().contains("aphrodite.toml"))
}
