//! Where the record machine is.

/// The three states of a git-log parse.
///
/// `Outside` is "no record open", `AwaitingSubject` is "a header was seen and its
/// subject has not arrived", `HaveSubject` is "the record is complete". The hash is
/// carried from state to state so the subject can be attributed to the commit it
/// actually belongs to - which is the whole difference between this machine and the
/// forward search it replaced. It is not a parser of the commit body: nothing else
/// about the record is retained.
pub(crate) enum Enum {
	Outside,
	AwaitingSubject { Hash:String },
	HaveSubject { Hash:String, Subject:String },
}
