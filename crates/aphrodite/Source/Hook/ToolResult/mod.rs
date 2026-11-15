//! Tool-result compression.
//!
//! `Entry/` is the public surface (three signatures over one pipeline),
//! `Inner/` is the gate sequence itself, and the rest are the pieces it calls:
//! type resolution, telemetry recording, written-path inference. The payload
//! shape produced here is what the Hermes bridge hands to the model, so every
//! field is a contract.

pub mod Classify;
pub mod Entry;
pub mod Inner;
pub mod Path;
pub mod Record;
