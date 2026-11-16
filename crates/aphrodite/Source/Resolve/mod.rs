//! CCR marker resolution - the read half of the marker wire format.
//!
//! Port of `plugins/aphrodite/_resolve.py`. A stored hash comes back to the model in two
//! shapes - an explicit retrieve, which must return the content the hash names, and
//! auto-expand, which may only inline it when it fits - and both land here. So this module
//! owns the whole answer to "what does this marker mean now".
//!
//! The tree separates the questions the resolver asks, because each one fails in a different
//! direction:
//!
//! - `Expand/` - the public entry point: expand a hash through every nesting level, and
//!   record the retrieval metric while doing it.
//! - `One/` - one hash, one store lookup, no unpacking.
//! - `Recursive/` - the cycle-safe walk over nested markers, and its size-bounded
//!   auto-expand variant.
//! - `Traversal/` - the state ONE expansion owns: its cache, its path, its budget. The walk
//!   cannot reach the rest of the process state, which is what makes the guard per-expansion
//!   rather than a convention.
//! - `Expansion/` - what a walk produced: the entry as loaded, or the entry fully expanded.
//! - `Halt/` - why it produced nothing: the hash is absent, or a bounded expansion abandoned
//!   its output.
//! - `Nested/` - what one occurrence became: replaced, left as written, or abandoned.
//! - `Filter/` - narrowing resolved content to the lines a query asked for, with `Match/` as
//!   the line scan and `Fold/` as the case fold it applies (the retrieve endpoint filters
//!   through the same scan).
//! - `Parse/` - the marker grammar: the two delimiters, hash extraction, the occurrence the
//!   scanner produces, and the tolerance policy for text that is not a usable reference.
//! - `Test/` - the battery, split by area.
//!
//! Nothing here compresses, rewrites or re-stores content. Resolution is a pure read, and the
//! one bug this module exists to prevent is a resolve that silently alters the bytes it was
//! asked to return.

pub mod Expand;
pub mod Expansion;
pub mod Filter;
pub mod Halt;
pub mod Nested;
pub mod One;
pub mod Parse;
pub mod Query;
pub mod Recursive;
pub mod Traversal;

#[cfg(test)]
mod Test;
