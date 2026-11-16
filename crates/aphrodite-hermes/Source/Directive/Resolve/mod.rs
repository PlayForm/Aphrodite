//! Where provisioning lands: the precedence chain over the caller's home, the
//! loader's exact-directory override, and the shared runtime home.
//!
//! One leaf per candidate - `Explicit/`, `Loader/`, `Home/`, `Runtime/` - each
//! answering "is this candidate the one?" as an `Option`, so `Fn/` composes them
//! with `or_else` and position is the only priority that exists.

pub mod Explicit;
pub mod Fn;
pub mod Home;
pub mod Loader;
pub mod Runtime;
