//! The backgrounding decision, split by what each hook can observe.
//!
//! Three questions, three owners: `Floor` asks whether the output is worth
//! tracking at all, `Command` asks which rule matched, and `Plan` is the answer -
//! the identity and the label the task will carry. `Entry` holds the two public
//! predicates and composes them, so a divergence between the pre-execution hook and
//! the post-execution one stays a difference of inputs rather than a difference of
//! rules.
//!
//! The split exists because the table has one owner - a second copy of it is how a
//! command starts backgrounding on the pre-hook and not the post-hook.

pub mod Command;
pub mod Entry;
pub mod Floor;
pub mod Plan;
