//! The target root every event from this crate carries.

/// Target root every event from this crate carries.
///
/// One constant, so a filter written by an operator
/// (`RUST_LOG=aphrodite_hermes=debug`), the fmt layer's own target filter, and
/// every emission site can never drift apart. It is a module path root on
/// purpose: `tracing_subscriber::filter::Targets` matches a target that equals
/// this string or continues it with `::`, so one constant covers the whole
/// crate.
pub(crate) const TARGET:&str = "aphrodite_hermes";
