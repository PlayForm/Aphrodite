//! TOML-driven preview template rendering (pre-Rust port).
//!
//! Port of `plugins/aphrodite/_core/template.py::_render_template`: when
//! `[templates.preview.<family>.<ctype>]` format strings are configured they
//! render instead of the hardcoded per-arm preview builders. Resolution per
//! content type: `family[ctype]` → `[templates.reverse][ctype]` mapped key →
//! `family[_default]` → `None` (the caller falls back to the hardcoded
//! builder). `{variable}` substitution mirrors the pre-Rust variable set;
//! unknown variables degrade to empty string - never a panic.
//!
//! Registration mirrors the `Preview::State` `Set`
//! pattern: a process-global registry set by `ConfigurationLoader::apply_templates`
//! (dylib path), `main.rs` (engine binary), and the proxy `/reload` handler
//! (hot-reload).
//!
//! The tree follows the render itself: `State/` is the registry (the singleton,
//! its storage, its accessor), `Set/`/`Get/` write and read the `[templates]`
//! section, `Family/` does the same for the model-family selector,
//! `Render/` is the one public entry point, `Vars/` builds the variable set
//! (one file per variable family), `Extract/` derives the values from content,
//! `Substitute/` performs the `{name}` replacement and `Guard/` serialises the
//! tests that mutate the registry.
//!
//! Nothing here may gain behaviour: an unconfigured template must leave the
//! caller on the hardcoded builders byte for byte, and a malformed template or
//! an unknown variable must degrade to a shorter string, never to a panic.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Template::Family::Get::Fn::Fn`), so the file that defines an item is the
//! only place it is defined and there is exactly one way to spell it.

pub mod Extract;
pub mod Family;
pub mod Get;
pub mod Render;
pub mod Set;
pub mod State;
pub mod Substitute;
pub mod Vars;

#[cfg(test)]
pub mod Guard;

#[cfg(test)]
mod Test;
