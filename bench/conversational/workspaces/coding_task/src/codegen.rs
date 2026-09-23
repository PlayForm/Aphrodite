use crate::parser::{ParserError, Token};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CodegenError {
    UnknownToken(Token),
    InvalidProgram(String),
    MissingInput,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::UnknownToken(t) => write!(f, "unknown token: {t:?}"),
            CodegenError::InvalidProgram(msg) => write!(f, "invalid program: {msg}"),
            CodegenError::MissingInput => write!(f, "missing input"),
        }
    }
}

pub fn generate(tokens: &[Token]) -> Result<String, CodegenError> {
    if tokens.is_empty() {
        return Err(CodegenError::MissingInput);
    }
    let mut out = String::from("llvm:\n");
    for t in tokens {
        match t {
            Token::Ident(name) => out.push_str(&format!("  load {name}\n")),
            Token::Int(n) => out.push_str(&format!("  const {n}\n")),
            Token::Plus => out.push_str("  add\n"),
            Token::Semicolon => out.push_str("  end_stmt\n"),
        }
    }
    Ok(out)
}
