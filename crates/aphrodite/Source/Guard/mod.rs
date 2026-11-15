//! Guard - the security boundary in front of every management route.
//!
//! Three independent checks run in this order: the TCP peer must be loopback,
//! its `Host` header must name a loopback address (peer-IP alone is not
//! DNS-rebinding-safe), and - when `APHRODITE_MGMT_TOKEN` is set - the request
//! must carry that token as a bearer. `/health` sits outside all three so an
//! external load-balancer probe still works; `/metrics` is exempt from the
//! bearer check only, because a Prometheus scraper cannot be given one.
//!
//! Each leaf pairs a pure decision function with a thin axum middleware that
//! only translates the decision's `Result` into a status code. The split is
//! deliberate: the decisions are unit-testable without an axum `Request`, and a
//! middleware that ever grows logic of its own is a defect, not a convenience.
//! Weakening any check here reopens a documented hole - see `SECURITY.md`.

pub mod Bearer;
pub mod Host;
pub mod Loopback;
pub mod Token;

#[cfg(test)]
pub mod Test;
