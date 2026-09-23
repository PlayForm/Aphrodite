use crate::{error::Error, parser::Token};

pub fn generate(tokens:&[Token]) -> Result<String, Error> {
	if tokens.is_empty() {
		return Err(Error::EmptyInput);
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
