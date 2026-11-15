//! The primitives every C ABI export calls before it touches session state:
//! the caller-string reader, the payload validator, and the rejection primitive
//! the validator is built from.
//!
//! Nothing here is exported itself, and nothing here renders a response - the
//! panic boundary, the error conversion, the serialisation and the ownership
//! transfer all live in `ABI::Boundary`. What is left is what belongs to the
//! pointer and the payload rather than to the response: what did the caller hand
//! over (`Cstr`), may this export serve it (`Content`), and how is a refusal
//! expressed without putting its predicate back at the call site (`Reject`).

pub mod Content;
pub mod Cstr;
pub mod Reject;
