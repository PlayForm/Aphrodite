//! Boot - the startup pipeline: arguments in, live proxy listeners out.
//!
//! `Aphrodite` has two ways to start - an `aphrodite.toml` with one
//! `[[proxies]]` entry per listener, or a single CLI-invoked proxy - and they
//! converge on the same steps: assemble the configuration, install the tracing
//! subscriber, bind every listener before any of them starts serving, then serve
//! until a shutdown signal drains them. Those steps live in `Configuration`,
//! `Listen`, `Route` and `State`; [`Fn`] is the entry the binary calls.
//!
//! Nothing here is a second copy of `crate::Proxy` - that module is the
//! request-handling engine, and this tree only decides when and where it runs.

mod Banner;
mod Configuration;
mod Fn;
mod Listen;
mod Route;
mod Run;
mod State;

pub use Banner::Announce;
pub use Fn::Fn;
