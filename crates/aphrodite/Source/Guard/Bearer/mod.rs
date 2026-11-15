//! Management-route authentication: the bearer-token gate.

pub mod Fn;
pub mod Middleware;

pub use Fn::CheckBearerToken;
pub use Middleware::RequireMgmtToken;
