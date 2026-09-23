mod error;
mod parser;
mod codegen;

fn main() {
	let src = "let x = 1 + 2;";
	let tokens = parser::tokenize(src);
	match codegen::generate(&tokens) {
		Ok(code) => println!("{code}"),
		Err(e) => {
			eprintln!("codegen failed: {e}");
			std::process::exit(1);
		},
	}
}
