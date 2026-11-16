//! The Go extractor: functions and methods, including the receiver-paren fix that
//! keeps a method's real parameters in the reported signature.

use crate::StructExtract::Extract::Fn as Structure;

#[test]
fn TestExtractGo() {
	let Code = "func main() {\n}\n\nfunc (s *Server) Start(addr string) error {\n}\n";
	let r = Structure(Code, "go");
	assert!(r.contains_key("fns"));
}

/// A method's receiver paren `(s *Server)` must not be mistaken for the
/// method's actual parameter list - `ParamsStart` used to find the
/// FIRST `(` on the line (the receiver's), reporting
/// `func Start(s *Server)` instead of `func Start(addr string)`.
#[test]
fn TestExtractGoMethodReportsRealParamsNotReceiver() {
	let Code = "func (s *Server) Start(addr string) error {\n}\n";
	let r = Structure(Code, "go");
	let Fns = &r["fns"];
	let Sig = Fns
		.iter()
		.find(|s| s.contains("Start"))
		.expect("Start method should be extracted");
	assert!(Sig.contains("addr"), "signature should show the real params: {Sig}");
	assert!(
		!Sig.contains("*Server"),
		"signature should NOT show the receiver as if it were a param: {Sig}"
	);
}
