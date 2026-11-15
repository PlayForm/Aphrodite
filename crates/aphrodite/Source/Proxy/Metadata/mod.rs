//! The metadata block that accompanies a compressed marker.
//!
//! Built per content family: the facts worth reporting about a build log are
//! not the facts worth reporting about a JSON payload. It is the model's map of
//! what it is no longer holding, so a misleading block is a bug. `Fn` is the
//! dispatcher, each family is its own file, and `Field` is the typed value every
//! producer returns instead of appending formatted strings to a caller's vector.

pub mod Build;
pub mod Code;
pub mod Diff;
pub mod Error;
pub mod Field;
pub mod Fn;
pub mod Git;
pub mod Go;
pub mod JS;
pub mod JSON;
pub mod Linter;
pub mod Log;
pub mod Python;
pub mod Rust;
pub mod Text;
