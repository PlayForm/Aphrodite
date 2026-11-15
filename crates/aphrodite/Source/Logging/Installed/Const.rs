//! The once-only guard for subscriber installation.

use std::sync::OnceLock;

/// The outcome of the one installation attempt, memoised so a later call is a
/// no-op and a repeated failure does not rebuild the subscriber.
///
/// `OnceLock` rather than an `AtomicBool`: the first caller's RESULT has to be
/// visible to every later caller, and two threads must not race two
/// `try_init` calls - the loser would report a startup error for a stack that
/// is in fact installed.
pub(crate) static Installed:OnceLock<Result<(), String>> = OnceLock::new();
