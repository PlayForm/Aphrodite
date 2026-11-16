//! Proxy liveness and the build stamp: the port pair, the TCP probe the
//! stats/health paths use, and which image answered.

pub mod Build;
pub mod Port;
pub mod Proxy;

#[cfg(test)]
mod Test;
