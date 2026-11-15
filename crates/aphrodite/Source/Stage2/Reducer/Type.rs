//! The reducer shape - the one signature every Stage-2 reducer has.
//!
//! A reducer is the whole contract between the registry and a content family:
//! it is handed the raw content and returns either a smaller replacement or
//! `None`. `None` is not an error - it means "not my type, or nothing worth
//! saving", and the caller then leaves the original content alone.

/// A reducer: content in, reduced content out, `None` when the content is not
/// the family this reducer handles or when the reduction would not help.
///
/// Deliberately a plain function pointer and not a trait object: the registry
/// is rebuilt per call from statics, so a `fn` pointer keeps that map the only
/// allocation involved. A trait object would move the dispatch cost onto every
/// lookup for no benefit - reducers take no state and never differ per session.
pub type Reducer = fn(&str) -> Option<String>;
