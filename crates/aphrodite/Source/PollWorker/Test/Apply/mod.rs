//! What a poll does to a task, as distinct outcomes rather than a boolean.
//!
//! Three facts have to stay separable: a poll that names no task of ours (not
//! consumed), one that moved a running task along (consumed, nothing stored), and one
//! that finished it (consumed, output moved out). The first is `Apply::Fn`'s `None`; the
//! other two are the two `Outcome` variants, and each owns exactly what its case has -
//! so a completion cannot be prepared from a progress result, and a progress result
//! cannot carry a buffer.

mod Transitions;
