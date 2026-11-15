//! The turn counter and the conversation-index cap.

pub mod Fn;

pub(crate) use self::Fn::ConvIndexCap;
pub use self::Fn::NextTurn;
