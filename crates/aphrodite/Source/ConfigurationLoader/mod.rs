//! TOML config loader - port of `plugins/aphrodite/_core/config.py`.
//!
//! Priority: env var > `aphrodite.toml` > hardcoded default, resolved per
//! knob. Search paths: cwd, then `~/.hermes/aphrodite/` - the config is NEVER
//! resolved relative to the binary (or the plugin dir).
//!
//! The tree is split by verb, because those verbs fail differently:
//! `Configuration::Load` finds and parses a file (a broken file degrades to
//! defaults), `Configuration::Resolve` reads one knob (an unusable value falls
//! through to the default), and `Configuration::Apply` writes a whole section
//! into `AphroditeState` (nothing fails; missing directories substitute
//! builtins). `Warn` holds the single diagnostic that must survive a host
//! process with no tracing subscriber.
//!
//! This file declares and re-exports only.

pub mod Configuration;
mod Warn;

#[cfg(test)]
mod Test;

/// The parse-failure warning, re-exported for `ABI::Reload`, which reports a
/// broken config on the hot-reload path.
pub(crate) use Warn::Fn::WarnParseFailure;
