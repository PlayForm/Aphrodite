//! Non-empty environment text.

/// A non-empty environment value, or `None`.
///
/// An empty value counts as UNSET. `std::env::var(...).ok()` cannot express that: a
/// shell profile exporting `APHRODITE_API_URL=` answers `Some("")`, which then wins the
/// precedence chain over the document and produces a proxy pointed at no upstream - a
/// configuration that looks applied and cannot serve a request. `Home::Environment::Path`
/// already applies this rule to paths; this is the same rule for the string knobs.
///
/// The distinction this keeps is the one that matters to a resolution chain: "the
/// operator named a value" versus "the variable exists and says nothing".
pub fn EnvironmentText(Var:&str) -> Option<String> { std::env::var(Var).ok().filter(|Value| !Value.is_empty()) }
