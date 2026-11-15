//! Compression thresholds: the value object, its resolver, and the
//! compiled-in defaults.
//!
//! Precedence is env var > `[compression]` TOML > default, resolved once at
//! startup and again on every reload. The value object, the resolver and the
//! constants are separate leaves because only the resolver knows the order and
//! only the constants are stable across releases.

pub mod Const;
pub mod Resolve;
pub mod Struct;
