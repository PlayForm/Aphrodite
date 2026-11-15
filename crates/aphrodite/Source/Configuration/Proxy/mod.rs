//! `aphrodite.toml` - the multi-proxy configuration model.
//!
//! Every struct here mirrors one TOML table, and the field names ARE the wire
//! format: they are what an operator writes in `aphrodite.toml` and what
//! `serde` deserialises. They stay `snake_case` for that reason alone.
//!
//! All knobs are `Option`, because "absent" is a distinct state from "zero"
//! or "false" - the resolver in `MultiConfig::resolve` only overrides what was
//! actually written.

pub mod Compression;
pub mod Defaults;
pub mod Mode;
pub mod Multi;
pub mod Previews;
pub mod Prompts;
// `Proxy` inside `Proxy` is the table's own name: `[[proxies]]` entries are
// the domain, and the directory has to carry it. Renaming the leaf would
// hide that the struct is the entry itself.
#[allow(clippy::module_inception)]
pub mod Proxy;
