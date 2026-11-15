//! Registering the installed plugin with Hermes, as an outcome rather than a
//! success flag.
//!
//! The last step of setup: `hermes plugins enable aphrodite`, so Hermes picks the
//! loader up on its next start. Everything the plugin needs is already on disk by
//! the time this runs, which is what makes a failure here a registration problem
//! rather than an install problem.

pub mod Fn;
pub mod Outcome;
