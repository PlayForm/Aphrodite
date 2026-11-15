//! The FNV-1a 64-bit parameters.
//!
//! These are the published offset basis and prime, not tunables: changing
//! either silently changes every signature the turn telemetry has already
//! recorded, which would make a repeated call look like a new one.

/// FNV-1a 64-bit offset basis - the starting value every hash begins from.
pub(crate) const FnvOffset:u64 = 0xCBF2_9CE4_8422_2325;
/// FNV-1a 64-bit prime - the multiplier applied after each byte's XOR.
pub(crate) const FnvPrime:u64 = 0x0000_0100_0000_01B3;
