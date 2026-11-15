//! The `[previews] model_family` selector.
//!
//! Two leaves, one per direction. The selector is separate from the templates
//! because it can be set on its own: a configuration may switch family without
//! redefining any format string, and the two are registered by different keys
//! in the TOML.

pub mod Get;
pub mod Set;
