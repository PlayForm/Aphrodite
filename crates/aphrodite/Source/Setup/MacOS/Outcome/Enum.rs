//! What one step of the install answered.

/// What one step of the install answered.
pub enum Enum {
	/// The tool ran and succeeded, or there was nothing to do.
	Done,
	/// The tool is missing or failed; the line names the consequence.
	Warned(String),
}
