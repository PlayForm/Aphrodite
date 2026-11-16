//! Gatekeeper-safe artifact install for macOS.
//!
//! Every file setup puts on a macOS install path - the binary and both dylibs -
//! goes through this module, because the treatment is not optional: `fs::copy`
//! preserves the quarantine attribute and the stale install name, and Gatekeeper
//! kills the process that loads the result. It is not a general copy helper and
//! it is not called on any other platform; the non-macOS paths copy and chmod
//! directly.
//!
//! Each step answers an `Outcome`: `Done`, or `Warned` carrying the line to
//! print. `Fn` runs the steps, collects the outcomes in the order they ran, and
//! hands the warnings to one sink - so a step cannot fail silently, and no step
//! decides for itself whether anyone hears about it.

pub mod Copy;
pub mod Dylib;
pub mod Fn;
pub mod Fallback;
pub mod Outcome;
