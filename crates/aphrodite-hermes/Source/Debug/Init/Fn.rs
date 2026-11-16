//! The one subscriber this dylib installs - and the case where it installs none.
//!
//! `libaphrodite_hermes` is not a process. It is a cdylib `dlopen`ed into the
//! Hermes gateway (a Python process) and, in dev, into a Rust host that may
//! already own the global dispatcher. `tracing::subscriber::set_global_default`
//! would therefore be a hostile act: it either wins and silences the host's
//! logging, or loses and takes the whole plugin down with it. `try_init` is the
//! only correct install - it answers "already set" as an `Err`, which this
//! module discards without a word.
//!
//! Two consequences follow, and both are deliberate:
//!
//!   - A second init is a no-op twice over: `Once` keeps the second call from
//!     reaching `try_init` at all, and `try_init` itself would refuse anyway.
//!     Nothing panics, and nothing is logged ABOUT the logging setup - a dylib
//!     that announces its own subscriber in every gateway log is noise.
//!   - The per-session debug FILE is not a subscriber layer. It is written by
//!     `Debug::Log::Bridge` through the core sink, because it has to keep
//!     working when this install loses to a host subscriber. A layer-based file
//!     sink would silently stop producing `debug.<root>` in exactly that case,
//!     which would break the `aphrodite_debug` toggle's contract.
//!
//! Filter precedence is the one §4.5 fixes: `RUST_LOG`, then `APHRODITE_LOG`,
//! then the level the engine captured at config load (0 -> warn, 1 -> debug,
//! 2 -> trace). The fmt layer is target-restricted to this crate so it can
//! never duplicate a host's own diagnostics into the same stderr.

use tracing_subscriber::{
	EnvFilter,
	Layer as _,
	filter::{LevelFilter, Targets},
	layer::SubscriberExt,
	util::SubscriberInitExt,
};

use crate::Debug::Init::{Const::TARGET, Level, Ownership, State::Install};

/// Install the process-wide subscriber, once per process, and report who ended
/// up owning the dispatcher.
///
/// Idempotent by design and silent on every failure path: a host that already
/// installed a subscriber, a filter string that fails to parse, and a second
/// call from another thread all end in the same place - this crate logs through
/// whatever dispatcher is in force, and the per-session debug file keeps being
/// written regardless.
///
/// The answer is a value, not a side effect: `Installed` means this crate's fmt
/// layer (target-restricted to [`TARGET`]) owns the dispatcher, so an event of
/// ours is guaranteed to reach stderr. `Host` means the host got there first and
/// may drop this crate's events without a word - which is exactly the case
/// `Debug::Warn` has to know about to add its stderr fallback.
pub(crate) fn Fn() -> &'static Ownership::Enum::Enum {
	Install.get_or_init(|| {
		let Filter = std::env::var("RUST_LOG")
			.ok()
			.or_else(|| std::env::var("APHRODITE_LOG").ok())
			.filter(|Spec| !Spec.trim().is_empty())
			.and_then(|Spec| EnvFilter::try_new(&Spec).ok())
			.unwrap_or_else(|| EnvFilter::new(Level::Fn::Fn()));

		// OFF by default, TRACE for this crate's targets only: a host dispatcher
		// may route other crates' events to its own log, and re-printing them
		// here would double-report the host's diagnostics.
		let Only = Targets::new()
			.with_default(LevelFilter::OFF)
			.with_target(TARGET, LevelFilter::TRACE);

		// `try_init` reports "already set" as an error instead of panicking -
		// that error IS the host-subscriber case, and it is not a defect. It is
		// also the answer this function returns, which is the only way a caller
		// can learn which of the two happened.
		let Subscriber = tracing_subscriber::registry().with(Filter).with(
			tracing_subscriber::fmt::layer()
				.with_target(false)
				.with_writer(std::io::stderr)
				.with_filter(Only),
		);

		match Subscriber.try_init() {
			Ok(()) => Ownership::Enum::Enum::Installed,
			Err(_) => Ownership::Enum::Enum::Host,
		}
	})
}
