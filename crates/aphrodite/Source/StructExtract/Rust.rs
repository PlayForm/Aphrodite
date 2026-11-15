//! Rust extractor: `fn`, `struct`, `trait` and `impl` signatures.
//!
//! Records `fn name(params) -> ret` for every function, then `struct name`,
//! `trait name` and `impl name` in that order, subtracting each line from the shared
//! preview budget as it goes. It is not a Rust parser: it reads one line at a time and
//! never tracks generics, so `impl<T> Foo<T>` is reported as `Foo` and a signature
//! split across lines is only half-read. What must not drift is the prefix list - a
//! declaration form the list does not name is invisible to the preview, which is how
//! `pub(crate) async fn` used to disappear from every Rust marker.

use std::collections::HashMap;

use crate::StructExtract::{
	Format::{Floor::Fn as Floor, Parameters::Fn as Parameters, Signature::Fn as Signature},
	MaximumSignatureLength::Const as MaximumSigLength,
};

/// Append this Rust source's `Fns`, `Structs`, `Traits` and `Impls` lines to `Result`,
/// spending `Budget`.
///
/// The categories are collected in that order out of one budget, and the function
/// returns early between them the moment the budget is spent - so a file with many
/// functions reports no traits or impls at all rather than a fraction of each. A
/// category is inserted only when it produced a line. It is not a formatter: the lines
/// stored here are already cut to the signature cap.
pub(super) fn Fn(Content:&str, Result:&mut HashMap<String, Vec<String>>, Budget:&mut isize) {
	// fn (with return type)
	let mut Fns:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();

		let IsFn = (Lower.starts_with("fn ")
			|| Lower.starts_with("pub fn ")
			|| Lower.starts_with("async fn ")
			|| Lower.starts_with("pub async fn ")
			|| Lower.starts_with("pub(crate) fn ")
			// T10 (F8): `pub(crate) async fn` was previously invisible here -
			// the strip chain a few lines below already handles stripping
			// "pub(crate) " then "async " in either order, but the `IsFn`
			// prefix check itself only recognized the non-async
			// `pub(crate) fn ` form.
			|| Lower.starts_with("pub(crate) async fn "))
			&& Trimmed.contains('(');

		if IsFn {
			// Extract name and params
			let AfterFn = Trimmed
				.trim_start_matches("pub(crate) ")
				.trim_start_matches("pub ")
				.trim_start_matches("async ")
				.trim_start_matches("fn ");
			if let Some(Paren) = AfterFn.find('(') {
				let name = &AfterFn[..Paren];
				let Rest = &AfterFn[Paren..];
				let ParamsEnd = Rest.find(')').unwrap_or(Rest.len());
				let Params = &Rest[1..ParamsEnd];
				let Ret = if Rest[ParamsEnd..].contains("->") {
					Rest[ParamsEnd..]
						.split("->")
						.nth(1)
						.unwrap_or("")
						.split('{')
						.next()
						.unwrap_or("")
						.trim()
				} else {
					""
				};
				let ParamsTrunc = Parameters(Params);
				let RetStr = if Ret.is_empty() { String::new() } else { format!(" -> {}", Ret) };
				let s = format!("fn {}({}){}", name, ParamsTrunc, RetStr);
				let s = if s.len() > MaximumSigLength {
					Floor(&s, MaximumSigLength - 3).to_string() + "..."
				} else {
					s
				};
				let Slen = s.len();
				Fns.push(s);
				*Budget -= Slen as isize + 1;
			}
		}
	}
	if !Fns.is_empty() {
		Result.insert("fns".into(), Fns);
	}
	if *Budget <= 0 {
		return;
	}

	// struct
	let mut Structs:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		if (Lower.starts_with("struct ") || Lower.starts_with("pub struct ")) && !Trimmed.contains('(') {
			let name = Trimmed
				.trim_start_matches("pub ")
				.trim_start_matches("struct ")
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.next()
				.unwrap_or("?");
			let s = Signature("struct", name);
			let Slen = s.len();
			Structs.push(s);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Structs.is_empty() {
		Result.insert("structs".into(), Structs);
	}
	if *Budget <= 0 {
		return;
	}

	// trait
	let mut Traits:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		if Lower.starts_with("trait ") || Lower.starts_with("pub trait ") {
			let name = Trimmed
				.trim_start_matches("pub ")
				.trim_start_matches("trait ")
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.next()
				.unwrap_or("?");
			let s = Signature("trait", name);
			let Slen = s.len();
			Traits.push(s);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Traits.is_empty() {
		Result.insert("traits".into(), Traits);
	}
	if *Budget <= 0 {
		return;
	}

	// impl
	let mut Impls:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		if Lower.starts_with("impl ") || Lower.starts_with("impl<") {
			let name = Trimmed
				.trim_start_matches("impl")
				.trim_start_matches('<')
				.split(|c:char| c.is_whitespace() || c == '<' || c == '{')
				.find(|s| !s.is_empty())
				.unwrap_or("?");
			let s = Signature("impl", name);
			let Slen = s.len();
			Impls.push(s);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Impls.is_empty() {
		Result.insert("impls".into(), Impls);
	}
}
