//! Command-line surface: the `Aphrodite` invocation, its subcommands, and the
//! `setup` argument set.
//!
//! The clap declarations and the structs the rest of the crate consumes are
//! deliberately separate. `Command` carries the CLI shape (long flags, env
//! fallbacks, defaults); `SetupArgs` carries the values `setup` acts on, so
//! `Run`/`WritePluginYAML` never depend on clap.

pub mod Command;
pub mod Setup;
pub mod Struct;
