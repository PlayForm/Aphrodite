use std::fmt;

use crate::parser::Token;

/// Shared error type for the parser and codegen stages.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // variants are part of the shared error API; only EmptyInput is exercised by the demo CLI
pub enum Error {
	/// A token that was not expected/allowed at this point.
	UnexpectedToken(Token),
	/// Parser: an unterminated construct (e.g. string literal).
	Unterminated(String),
	/// Codegen: a program that cannot be lowered to IR.
	InvalidProgram(String),
	/// No input was provided.
	EmptyInput,
}

impl fmt::Display for Error {
	fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::UnexpectedToken(t) => write!(f, "unexpected token: {t:?}"),
			Error::Unterminated(s) => write!(f, "unterminated: {s}"),
			Error::InvalidProgram(msg) => write!(f, "invalid program: {msg}"),
			Error::EmptyInput => write!(f, "empty input"),
		}
	}
}

impl std::error::Error for Error {}
