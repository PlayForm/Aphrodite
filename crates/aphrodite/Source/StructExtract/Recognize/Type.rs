//! The recogniser shape - the one signature every declaration recogniser has.

use crate::StructExtract::Declaration::Enum::Enum as Declaration;

/// A recogniser: one line in, one verdict out, borrowing the line it read.
///
/// The higher-ranked lifetime is the whole contract. A recogniser may not allocate and may not keep
/// anything: the `&'a str` it is handed is the `'a` in the declaration it returns, so a verdict can
/// never outlive the line it describes and no recogniser can smuggle an owned copy into the preview.
/// The renderer is where the single allocation happens.
///
/// It is not a trait: every recogniser is a plain `fn`, they are listed as data in `Policy`, and a
/// trait object would move dispatch onto the per-line path for no gain.
pub(crate) type Recognize = for<'a> fn(&'a str) -> Declaration<'a>;
