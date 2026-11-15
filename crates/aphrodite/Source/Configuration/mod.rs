//! CLI configuration for aphrodite.
//!
//! Generic LLM proxy - works with any OpenAI-compatible API.
//! Cache and Token modes with CCR, tool relay, programmatic CCR.
//!
//! Two surfaces live here and must stay distinguishable: the **command line**
//! (`CLI`) and the **file** (`Proxy`). They meet at exactly one place -
//! `Proxy::Multi::Resolve::Fn::resolve`, which produces the CLI-shaped value
//! from the TOML document - so a knob that exists in both has one documented
//! precedence (env > TOML > default) instead of two.
//!
//! `Environment` holds the two env readers every resolution chain shares.
//!
//! This file declares only. A caller names the leaf
//! (`Configuration::Proxy::Mode::Enum::Enum`), so the file that defines an
//! item is the only place it is defined.

pub mod CLI;
pub mod Environment;
pub mod Proxy;

#[cfg(test)]
pub mod Test;

