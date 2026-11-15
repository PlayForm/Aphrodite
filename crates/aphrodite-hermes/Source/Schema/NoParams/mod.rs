//! The parameter block shared by every argument-less tool.
//!
//! One definition, so the `additionalProperties: false` guard cannot be lost by a
//! copy that forgets it.

mod Fn;

pub(crate) use Fn::Fn;
