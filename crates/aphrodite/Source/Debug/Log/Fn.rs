//! `Log`: a record at the process-global level.

use crate::Debug::Level::Get::Fn::Fn as Level;

/// Log at the process-global level (stateless call sites).
///
/// The stateless variant exists for code that has no `AphroditeState` in hand -
/// the template renderer, the marker formatter. It reads the mirror the config
/// loader published rather than threading a level through every signature.
pub fn Fn(Event:&str, Kv:&[(&str, String)]) { super::At::Fn::Fn(Level(), Event, Kv); }
