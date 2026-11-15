//! Hook-name routing: decode the request, prepare its effects, run it.
//!
//! `AphroditeDispatch` is one export serving thirteen commands, and the three
//! stages are separated because they happen on three different sides of the
//! session lock:
//!
//! - `Command/` - the request decoded from a hook name and its JSON args. Pure:
//!   no session, no I/O. Its variants are the whole routing table, so a name
//!   that is not in it is `Unknown` and the match in `Run` is exhaustive.
//! - `Prepared/` - a command whose external effects have already happened.
//!   Today that is exactly `prefetch`, whose file reads must not run under the
//!   session lock; every other command passes through untouched.
//! - `Decode/` - the hook name and args to a `Command`, including the payload
//!   cap that runs before anything else.
//! - `Prepare/` - a `Command` to a `Prepared`, performing the file reads.
//! - `Run/` - a `Prepared` to the JSON envelope, under the session lock.
//!
//! The order of those stages is the error precedence, and it is a contract:
//! handle decoding, then the JSON parse, then the payload cap, then the span,
//! then the file reads, then the handle's existence, then the command. Moving
//! the handle check earlier - the obvious "validate first" tidy-up - would
//! change which error wins and would stop the filesystem read from happening at
//! all for an invalid handle.

pub mod Command;
pub mod Decode;
pub mod Prepare;
pub mod Prepared;
pub mod Run;
