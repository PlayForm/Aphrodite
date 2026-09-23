#[derive(Debug, Clone, PartialEq)]
pub enum Token {
	Ident(String),
	Int(i64),
	Plus,
	Semicolon,
}

pub fn tokenize(input:&str) -> Vec<Token> {
	let mut tokens = Vec::new();
	for part in input.split_whitespace() {
		match part {
			"+" => tokens.push(Token::Plus),
			";" => tokens.push(Token::Semicolon),
			_ if part.parse::<i64>().is_ok() => tokens.push(Token::Int(part.parse().unwrap())),
			_ => tokens.push(Token::Ident(part.to_string())),
		}
	}
	tokens
}
