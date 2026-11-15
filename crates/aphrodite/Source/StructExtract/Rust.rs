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
	// `Err(())` is the spent-budget sentinel: it stops the scan exactly where the
	// `break` did, and the result is discarded because the budget, not the walk,
	// is the outcome.
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
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

		match IsFn {
			true => {
				// Extract name and params
				let AfterFn = Trimmed
					.trim_start_matches("pub(crate) ")
					.trim_start_matches("pub ")
					.trim_start_matches("async ")
					.trim_start_matches("fn ");
				match AfterFn.find('(') {
					Some(Paren) => {
						let name = &AfterFn[..Paren];
						let Rest = &AfterFn[Paren..];
						let ParamsEnd = Rest.find(')').unwrap_or(Rest.len());
						let Params = &Rest[1..ParamsEnd];
						let Ret = match Rest[ParamsEnd..].contains("->") {
							true => {
								Rest[ParamsEnd..]
									.split("->")
									.nth(1)
									.unwrap_or("")
									.split('{')
									.next()
									.unwrap_or("")
									.trim()
							},
							false => "",
						};
						let ParamsTrunc = Parameters(Params);
						let RetStr = match Ret.is_empty() {
							true => String::new(),
							false => format!(" -> {}", Ret),
						};
						let s = format!("fn {}({}){}", name, ParamsTrunc, RetStr);
						let s = match s.len() > MaximumSigLength {
							true => Floor(&s, MaximumSigLength - 3).to_string() + "...",
							false => s,
						};
						let Slen = s.len();
						Fns.push(s);
						*Budget -= Slen as isize + 1;
					},
					None => {},
				}
			},
			false => {},
		}
		Ok(())
	});
	match Fns.is_empty() {
		false => {
			Result.insert("fns".into(), Fns);
		},
		true => {},
	}
	match *Budget <= 0 {
		true => {
			return;
		},
		false => {},
	}

	// struct
	let mut Structs:Vec<String> = Vec::new();
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		match (Lower.starts_with("struct ") || Lower.starts_with("pub struct ")) && !Trimmed.contains('(') {
			true => {
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
			},
			false => {},
		}
		Ok(())
	});
	match Structs.is_empty() {
		false => {
			Result.insert("structs".into(), Structs);
		},
		true => {},
	}
	match *Budget <= 0 {
		true => {
			return;
		},
		false => {},
	}

	// trait
	let mut Traits:Vec<String> = Vec::new();
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		match Lower.starts_with("trait ") || Lower.starts_with("pub trait ") {
			true => {
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
			},
			false => {},
		}
		Ok(())
	});
	match Traits.is_empty() {
		false => {
			Result.insert("traits".into(), Traits);
		},
		true => {},
	}
	match *Budget <= 0 {
		true => {
			return;
		},
		false => {},
	}

	// impl
	let mut Impls:Vec<String> = Vec::new();
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		match Lower.starts_with("impl ") || Lower.starts_with("impl<") {
			true => {
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
			},
			false => {},
		}
		Ok(())
	});
	match Impls.is_empty() {
		false => {
			Result.insert("impls".into(), Impls);
		},
		true => {},
	}
}
