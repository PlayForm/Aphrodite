//! Where provisioning lands: the precedence chain over the caller's home, the
//! loader's exact-directory override, and the shared runtime home.
//!
//! Re-exported under its kind name so the joining module can call it as
//! `Resolve::Fn()`.

mod Fn;

pub(crate) use Fn::Fn;
