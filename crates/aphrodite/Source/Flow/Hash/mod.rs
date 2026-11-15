//! The turn-telemetry hash primitive.

pub mod Const;
pub mod Fn;

pub(crate) use self::Fn::Fnv1a;
