//! The one retry policy this proxy ships.
//!
//! At most three attempts, and only for connect-phase failures: a post-send
//! timeout may already have been accepted upstream, and blindly retrying a
//! non-idempotent `POST /v1/chat/completions` risks double token billing. With
//! the 300s per-attempt timeout, three attempts could hold a client for ~15
//! minutes, which is the ceiling this policy exists to keep.
//!
//! Broadening it - retrying a timeout, retrying a 5xx - is a separate decision
//! with a separate justification, not a knob to turn here.

use crate::Proxy::Handle::Proxy::Forward::Policy::Struct::Struct as ForwardPolicy;

/// Three attempts, 100ms doubling, jittered 0.75x-1.25x.
pub(crate) const Const:ForwardPolicy =
	ForwardPolicy { MaximumAttempts:3, Backoff:100, JitterFloor:0.75, JitterSpan:0.5 };
