//! The C ABI plumbing every export routes through: string marshalling and the
//! panic guard.
//!
//! Nothing here may be skipped at an export boundary. A panic that unwinds
//! across `extern "C"` aborts the host process (the Rust runtime's forced
//! abort), and a return value allocated one way but freed another corrupts the
//! allocator - both are process-fatal in a dylib loaded into Hermes' Python
//! process, which is why `guarded` and the `CString::into_raw` /
//! `CString::from_raw` pairing are non-negotiable.
//!
//! `Fn/` still carries four items (`cstr_to_string`, `to_c_string`,
//! `to_json_error`, `guarded`) where the house style wants four leaves -
//! `Guard::FromC`, `Guard::ToC`, `Guard::Error` and `Guard::Panic`, each with a
//! bare `Fn`. That split is blocked on `Hook::Call::{Fn,Encode}`, which names all
//! four and lives outside this partition.

pub mod Fn;
pub mod ToJSON;

#[cfg(test)]
mod Test;
