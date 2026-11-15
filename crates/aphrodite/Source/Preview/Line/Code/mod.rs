//! Code-shape line predicates.
//!
//! Source code is recognised by a small number of unmistakable lines - a
//! function signature, a type declaration, an include - and by a larger number
//! of statement lines that only vote. The detector needs both grades: one
//! strong line is enough to call content code, while statements are counted.
//! Splitting the two grades is the whole point of this module; a single
//! "looks like code" predicate would either miss plain scripts or claim prose.

pub mod FromImport;
pub mod FunctionSignature;
pub mod IncludeDirective;
pub mod LetAssignment;
pub mod StrongLine;
pub mod TypeDeclaration;
pub mod UseStatement;
pub mod VoteLine;
