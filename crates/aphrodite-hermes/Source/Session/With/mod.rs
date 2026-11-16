//! Run a closure against a session's state under the bounded lock.
//!
//! This group holds TWO entry points in one leaf (`Fn::with_session` and
//! `Fn::with_shared`), which is the one place the one-item-per-file rule is
//! still unpaid: the bare `Fn` name belongs to whichever of them owns the leaf,
//! so the split is `With::Session::Fn` and `With::Shared::Fn`. It is blocked on
//! the callers outside this crate's `Session/`, `Debug/`, `ABI/`,
//! `Configuration/`, `Health/`, `Guard/` and `Bin/` trees - `Hook/` and `Tool/`
//! name these two functions at ~40 sites - so the rename waits for that row.

pub mod Fn;
