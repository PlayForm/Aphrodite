//! The config-file hot-reload watcher, in four layers and one value.
//!
//! `Target` is what a watch is aimed at - the configuration file, resolved to an
//! absolute path, plus its directory. `Source` is the platform event source: it owns
//! the `notify` watcher and the channel its events arrive on, and it knows nothing about
//! configuration. `Debounce` collapses a burst of events into one settled moment.
//! `Apply` loads the file and writes the new thresholds into every live listener. `Loop`
//! is the controller that composes those three, and `Fn` is the production wiring - the
//! only part that needs a real filesystem watch.
//!
//! `Outcome` and `Stop` are the lifecycle: whether a watch is live is a value the caller
//! reads, and stopping it is a step in the shutdown sequence rather than something left
//! to process exit.
//!
//! The split exists so the controller can be exercised with a channel a test owns:
//! nothing below `Fn` touches `notify`, and nothing above `Apply` touches a file.
//!
//! Only the compression thresholds are hot-applied; everything else a config file
//! describes needs a restart.

pub mod Apply;
pub mod Debounce;
pub mod Fn;
pub mod Loop;
pub mod Outcome;
pub mod Source;
pub mod Stop;
pub mod Target;
