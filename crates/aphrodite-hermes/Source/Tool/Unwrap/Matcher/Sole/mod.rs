//! Whether an envelope is the SOLE field of its object.
//!
//! Two matchers collapse a payload field only when that field is the whole
//! object: a bare `{"success": true}` is a USER payload (collapsing it produced
//! `[text:1L 2B | ok]` and hid the entire result), and a multi-key
//! `{"success": "ok", "data": [...]}` must preview as itself rather than as the
//! word `ok`.
//!
//! The rule is a property of the OBJECT, not of the field a matcher reads, so it
//! is decided here and handed over as the narrowed object. Testing it inside the
//! matcher produced `filter(|_| Object.len() <= 1)` - an `Option::filter` whose
//! predicate ignores the value it filters, which is an `if` wearing a
//! combinator's clothes: it reads as a filter over the message, hides the branch
//! from every reader and every probe, and re-derives a fact about the object
//! after the object was already consulted.

pub mod Fn;
