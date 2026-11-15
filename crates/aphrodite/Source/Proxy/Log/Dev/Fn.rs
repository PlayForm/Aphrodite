//! Emit a dev-mode log line only when dev mode is on.
//!
//! Dev mode is a logging concern, not a request decision, so it has one
//! primitive instead of a `match State.Dev { true => ..., false => {} }` at
//! every site. `bool::then` keeps the closure lazy: the format arguments are
//! built only on the enabled path, which is what makes it safe to put a
//! `format!` or a preview render inside one.

/// Call `Line` when `Dev` is set.
pub(crate) fn Fn(Dev:bool, Line:impl FnOnce()) { let _ = Dev.then(Line); }
