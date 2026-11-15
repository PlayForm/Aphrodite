//! `Respond` - turn the upstream outcome into the client response.
//!
//! Three branches that must stay distinct: an SSE body is relayed chunk by
//! chunk and never buffered or cached; a buffered body is compressed when it is
//! a Chat Completions response and the CCR store is enabled; a transport error
//! becomes a 502 with a generic message. The upstream error string is recorded
//! server-side only - it can embed the upstream URL (02-F8).
//!
//! `Fn` branches once, on delivery shape, after the head is extracted; each
//! interpreter owns its own accounting, and neither can accidentally do the
//! other's job.

pub mod BadGateway;
pub mod Buffered;
pub mod Failure;
pub mod FinalBody;
pub mod Fn;
pub mod Status;
pub mod Stream;
pub mod Upstream;
