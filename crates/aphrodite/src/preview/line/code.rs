//! Code line predicates: strong signatures, type declarations, statements,
//! and the vote lines that decide the `code` shape.

/// `fn name(` / `def name(` / `func name(` style signature.
pub(crate) fn is_fn_style_sig(t: &str, kw: &str) -> bool {
	let rest = match t.strip_prefix(kw) {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.trim_start();
	let name_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	name_len > 0 && rest[name_len..].trim_start().starts_with('(')
}

/// `struct Name` / `enum Name` / `trait Name` (optionally `pub`-prefixed).
pub(crate) fn is_type_decl(t: &str) -> bool {
	let stripped = t.strip_prefix("pub ").unwrap_or(t);
	for kw in ["struct ", "enum ", "trait "] {
		if let Some(rest) = stripped.strip_prefix(kw) {
			let name_len = rest
				.trim_start()
				.chars()
				.take_while(|c| c.is_alphanumeric() || *c == '_')
				.count();
			if name_len > 0 {
				return true;
			}
		}
	}
	false
}

/// `#include <...>` / `#include "..."` / `#include<...>`.
pub(crate) fn is_include_directive(t: &str) -> bool {
	let rest = match t.strip_prefix("#include") {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.trim_start();
	rest.starts_with('<') || rest.starts_with('"')
}

/// Strong code-signature line: ONE such line is enough to call content code.
pub(crate) fn is_code_strong_line(t: &str) -> bool {
	is_fn_style_sig(t, "fn ")
		|| is_fn_style_sig(t, "def ")
		|| is_fn_style_sig(t, "func ")
		|| is_type_decl(t)
		|| t.starts_with("impl ")
		|| t.starts_with("class ")
		|| t.starts_with("#!")
		|| is_include_directive(t)
		|| t.strip_prefix("package ")
			.map(|r| r.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false))
			.unwrap_or(false)
}

/// `let x =` / `const X =` / `static X =` assignment (optional `mut`).
pub(crate) fn is_let_assign(t: &str) -> bool {
	let rest = match ["let ", "const ", "static "].iter().find_map(|p| t.strip_prefix(p)) {
		Some(r) => r,
		None => return false,
	};
	let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim_start();
	let name_len = rest
		.chars()
		.take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
		.count();
	name_len > 0 && rest[name_len..].trim_start().starts_with('=')
}

/// `use std::collections::HashMap;` (rust use statement ending in `;`).
pub(crate) fn is_use_statement(t: &str) -> bool {
	let rest = match t.strip_prefix("use ") {
		Some(r) => r,
		None => return false,
	};
	let path_len = rest
		.chars()
		.take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
		.count();
	path_len > 0 && rest[path_len..].starts_with(';')
}

/// `from x import y` (python).
pub(crate) fn is_from_import(t: &str) -> bool {
	let rest = match t.strip_prefix("from ") {
		Some(r) => r,
		None => return false,
	};
	let mod_len = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
	mod_len > 0 && rest[mod_len..].trim_start().starts_with("import")
}

/// Code statement-line vote: `use x::y;`, `let x =`, `import x`,
/// `from x import y`, `return ...`, `println!`, `print(`, `echo ...`.
pub(crate) fn is_code_vote_line(t: &str) -> bool {
	is_use_statement(t)
		|| is_let_assign(t)
		|| is_from_import(t)
		|| t.strip_prefix("import ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false))
			.unwrap_or(false)
		|| t.strip_prefix("return")
			.map(|r| r.chars().next().map(|c| c.is_whitespace()).unwrap_or(false))
			.unwrap_or(false)
		|| t.starts_with("println!")
		|| t.starts_with("print(")
		|| t.strip_prefix("print")
			.map(|r| r.trim_start().starts_with('('))
			.unwrap_or(false)
		|| t.strip_prefix("echo ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '$').unwrap_or(false))
			.unwrap_or(false)
}
