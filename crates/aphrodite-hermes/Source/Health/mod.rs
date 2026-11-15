//! Proxy liveness: the port pair and the TCP probe the stats/health paths use.

pub mod Port;
pub mod Proxy;

#[cfg(test)]
mod Test;
