//! The one-line startup banner: build identity and listener count.
//!
//! Emitted after the configuration is assembled and before any socket is bound,
//! so a log reader can tell which build a listener belongs to even when a bind
//! then fails. The values are baked in at compile time by `build.rs`; an absent
//! one prints `?` rather than an empty field.

/// Log the build identity and how many proxy listeners are about to start.
pub fn Announce(Listeners:usize) {
	tracing::info!(
		"aphrodite v{} ({}{}) • {} • {}",
		option_env!("APHRODITE_VERSION").unwrap_or("?"),
		option_env!("APHRODITE_GIT_HASH").unwrap_or("?"),
		option_env!("APHRODITE_PROFILE").map(|p| format!(", {p}")).unwrap_or_default(),
		option_env!("APHRODITE_BUILD_DATE").unwrap_or("?"),
		option_env!("APHRODITE_TARGET").unwrap_or("?"),
	);

	tracing::info!("starting {} proxy listener(s)", Listeners);
}
