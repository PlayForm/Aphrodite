//! grep/ripgrep hit-line predicates.
//!
//! A grep hit is `path:line:match`. Two predicates read that shape: the grep
//! arm needs a file-ish path, the search arm does not. They share the split, so
//! the split lives once here - two copies would drift and one arm would start
//! counting lines the other rejects.

mod SplitPath;

pub mod Hit;
pub mod Search;
