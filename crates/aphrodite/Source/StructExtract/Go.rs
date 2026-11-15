//! Go extractor: `func` and `type ... struct` signatures.
//!
//! Reads the content line by line and records `func Name(params)` for every
//! function or method, then `type Name` for every struct, subtracting each line from
//! the shared preview budget as it goes. It is not a Go parser: it never balances
//! braces, so a `func` that appears inside a string literal is recorded like a real
//! declaration - the accepted cost of staying allocation-light on the preview path.
//! What must not drift is the receiver handling: a method whose receiver is reported
//! as its parameter list tells the model the wrong signature.

use std::collections::HashMap;

use crate::StructExtract::{
	Format::{Floor::Fn as Floor, Parameters::Fn as Parameters, Signature::Fn as Signature},
	MaximumSignatureLength::Const as MaximumSigLength,
};

/// Append this Go source's `Fns` and `Types` lines to `Result`, spending `Budget`.
///
/// Each signature subtracts its own length plus one separator from the budget, and
/// the scan stops the moment the budget is spent, so the `Types` pass only runs when
/// the `Fns` pass left room. A category is inserted only when it produced a line,
/// and the function returns early once the budget is gone rather than emitting an
/// empty remainder. It is not a formatter: the lines stored here are already cut to
/// `MaximumSigLength`, so a caller must not re-truncate them.
pub(super) fn Fn(Content:&str, Result:&mut HashMap<String, Vec<String>>, Budget:&mut isize) {
	let mut Fns:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		if Trimmed.starts_with("func ") && Trimmed.contains('(') {
			// func Name(...) or func (r *Receiver) Name(...)
			let AfterFunc = &Trimmed["func ".len()..];
			// T10 (F8): for a method, the receiver's own `(...)` comes before
			// the method's real parameter list - searching for the first `(`
			// in the whole line (as the old code did) finds the RECEIVER's
			// paren, not the params, so `func (s *Server) Start(addr string)`
			// used to report `func Start(s *Server)` instead of
			// `func Start(addr string)`. Search for the params paren only
			// after skipping past the receiver's closing `)`.
			let ReceiverEnd = if AfterFunc.starts_with('(') {
				AfterFunc.find(')').map(|i| i + 1)
			} else {
				None
			};
			let name = match ReceiverEnd {
				Some(End) => AfterFunc[End..].trim_start().split('(').next().unwrap_or("?"),
				None => AfterFunc.split('(').next().unwrap_or("?"),
			};
			let SearchFrom = match ReceiverEnd {
				// Absolute offset into `Trimmed`: "func ".len() + receiver_end
				Some(End) => "func ".len() + End,
				None => 0,
			};
			let ParamsStart = SearchFrom + Trimmed[SearchFrom..].find('(').unwrap_or(0);
			let ParamsEnd = Trimmed[ParamsStart..].find(')').unwrap_or(0);
			let Params = if ParamsEnd > 1 {
				&Trimmed[ParamsStart + 1..ParamsStart + ParamsEnd]
			} else {
				""
			};
			let s = format!("func {}({})", name, Parameters(Params));
			let STrunc = if s.len() > MaximumSigLength {
				Floor(&s, MaximumSigLength - 3).to_string() + "..."
			} else {
				s
			};
			let Slen = STrunc.len();
			Fns.push(STrunc);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Fns.is_empty() {
		Result.insert("fns".into(), Fns);
	}
	if *Budget <= 0 {
		return;
	}

	let mut Types:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		if Trimmed.starts_with("type ") && Trimmed.contains("struct") {
			let name = Trimmed["type ".len()..].split("struct").next().unwrap_or("?").trim();
			let s = Signature("type", name);
			let Slen = s.len();
			Types.push(s);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Types.is_empty() {
		Result.insert("types".into(), Types);
	}
}
