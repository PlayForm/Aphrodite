//! Tests for the setup install payload.
//!
//! Two areas, split by what they read. `PluginShim/` pins what the embedded
//! `__init__.py` loader must contain and that the embedded copy has not drifted
//! from the live plugin; `Checksum/` exercises the download checksum against a
//! real release. The shim tests are hermetic, the checksum test touches the
//! network and is `#[ignore]`d so the default suite stays offline.

pub mod Checksum;
pub mod PluginShim;
