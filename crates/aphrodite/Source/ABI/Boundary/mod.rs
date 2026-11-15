//! The C ABI boundary: one adapter every export routes through.
//!
//! Before this directory existed, every export repeated the same five steps -
//! read the caller's C string, validate it, run the operation, convert an
//! `anyhow::Error` into `{"error": ...}`, hand the `CString` over - and the
//! repetition was where the shapes diverged. Now each step has one home:
//!
//! - `Answer/` is what a successful operation produces: a JSON envelope or raw
//!   text. Two shapes, because the ABI really has two (`AphroditeRetrieve` and
//!   the other text exports answer with the payload itself, not a wrapper).
//! - `Run/` is the panic boundary. It catches an unwind and turns it into
//!   `{"error":"internal error: panicked"}`, which is the message the exports
//!   that do not take a handle have always answered.
//! - `State/` adds the session: it decodes the handle and runs the operation
//!   under that session's own lock. `State::Fn` goes through
//!   `Handle::Lookup::WithState`, whose inner `catch_unwind` answers
//!   `"internal error: hook panicked"`; `State::Bare` does not, so the panic
//!   reaches `Run` and keeps the `"internal error: panicked"` wording the
//!   read-shaped exports have always had.
//! - `Failure/` builds the `{"error": ...}` envelope from an `anyhow::Error`.
//! - `Render/` composes failure and transfer: an outcome, right or wrong,
//!   becomes the pointer the caller receives.
//! - `Transfer/` is the ownership handover, and the one place a payload with an
//!   interior NUL is neutralised before it can truncate the C string.
//!
//! An export body is therefore one fallible operation: it names what it needs
//! and returns `Ok(Answer::Enum::Envelope(..))`, with no envelope, no pointer and
//! no panic handling of its own.

pub mod Answer;
pub mod Failure;
pub mod Handle;
pub mod Render;
pub mod Run;
pub mod State;
pub mod Transfer;
