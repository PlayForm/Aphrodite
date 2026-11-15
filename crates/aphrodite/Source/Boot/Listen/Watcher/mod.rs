//! The config-file hot-reload watcher, in three layers.
//!
//! `Source` is the platform event source: it owns the `notify` watcher and the
//! channel its events arrive on, and it knows nothing about configuration.
//! `Debounce` collapses a burst of events into one tick. `Apply` loads the file
//! and writes the new thresholds into every live listener. `Loop` is the
//! controller that composes the three, and `Fn` is the production wiring - the
//! only part that needs a real filesystem thread.
//!
//! The split exists so the controller can be exercised with a channel a test
//! owns: nothing below `Fn` touches `notify`, and nothing above `Apply` touches
//! a file.
//!
//! Only the compression thresholds are hot-applied; everything else a config
//! file describes needs a restart.

pub mod Apply;
pub mod Debounce;
pub mod Fn;
pub mod Loop;
pub mod Source;
