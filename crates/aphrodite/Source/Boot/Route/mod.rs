//! The axum router, one leaf per route group.
//!
//! `Build` is the only place the group routers are combined, and the two layers
//! it applies are the security boundary: `RequireMgmtToken` on the restricted
//! groups and `LoopbackOnly` on everything except `/health`. Applying either
//! layer inside a group instead would leave the other groups unguarded, which is
//! the one mistake this module exists to make hard.

mod Build;
mod CCR;
mod Chat;
mod Health;
mod Management;
mod Metrics;
mod Relay;
mod Retrieve;

pub use Build::Build;
