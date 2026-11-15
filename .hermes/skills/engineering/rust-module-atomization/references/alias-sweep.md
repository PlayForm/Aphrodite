# The alias sweep, mechanically

Four passes over the tree, each one cheap to re-run, driven by what the
compiler says rather than by a pattern you hoped was complete.

## 1. Find the aliases (and keep the map)

A phase-2 alias is a `use` statement whose line ends with a marker comment:

```rust
pub use Body::Capped::Struct::Struct as CappedBody; // TODO(house-style): drop after sweep
```

Do not match the alias with a line regex. A facade may hold a brace group -
`pub use Run::{Entry::Fn::Fn as run, Error::Enum as SetupError};` - or a group
spread over several lines whose closing `};` carries the marker. Parse every
`use ...;` statement (start at `^\s*(pub(...)?\s+)?use `, then scan to the first
`;` at bracket depth zero), flatten its tree to leaves, and treat the statement
as aliases when its trailing text carries the marker.

The map you need is `(crate, facade_module_segments, local_name) -> leaf_segments`.
The facade module is the file's own module path (`Preview/Builder/mod.rs` is
`("Preview","Builder")`).

## 2. Delete the statements

Delete the whole statement, its trailing comment, and - walking backwards over
the preceding lines - any `///` block or `#[...]` attribute that documented it.
Leaving the doc block behind makes the module unparseable with
`error: expected item after attributes`, and a module that does not parse makes
every path through it unresolved (the errors then read
`cannot find Family in Template`, which looks like a different bug).

## 3. Repoint the imports

Flatten every other `use` statement in both crates and resolve each leaf's
prefix:

- `crate::` and a bare head → the file's own crate root (uniform paths make a
  bare head module-relative; resolve it against the file's module).
- `super::`/`self::` → relative to the file's module.
- the package name (`aphrodite`) → the other crate's root.

A leaf whose `(crate, resolved_prefix, local_name)` is in the map is an alias:
emit one absolute line for it.

```
use crate::Proxy::Body::Capped::Struct::Struct as CappedBody;
```

Keep the consumer's local name (`as CappedBody`) so its bare uses keep
compiling - the import moved, the name did not. Rebuild the statement as one
line per leaf instead of editing an item inside the group: in
`use crate::Preview::{Input::Input, Line::Error}` the replacement's base is the
group prefix, not the crate root, and patching in place silently yields
`crate::Struct::Struct`.

Render the absolute path with the head the source wrote. `crate::` inside
`Source/Boot/**` is the BINARY's root; those files reach the engine crate as
`aphrodite::`, so a rewritten path there must keep the `aphrodite::` head.

## 4. Repoint the qualified paths

Outside use statements, rewrite `<facade>::<old>` → `<facade>::<leaf>`, but only
where the match does not sit inside a use statement (the statements were already
handled) and with a `(?<![\w])` lookbehind - `(?<![\w:])` also refuses
`crate::Facade::old`, which is the common case.

Two forms need their own pattern:

- the facade's own subtree: `(?:super::)+old` → the absolute leaf path;
- the other crate: `aphrodite::<facade>::old` → `aphrodite::<facade>::<leaf>`.

## 5. Close the three gaps the compiler reports

| Diagnostic                             | Cause                                             | Fix                                                                                                             |
| -------------------------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `cannot find function X in this scope` | a glob import (`use super::*;`) carried the alias | insert `use crate::<facade>::<leaf> as X;` per reported name, at the file's glob anchor                         |
| `module X is private` (E0603)          | a private `mod X;` on the leaf path               | `pub(crate) mod X;`, or `pub mod` when the binary target names it                                               |
| `unused import: crate::Proxy::*`       | the explicit imports made the glob redundant      | delete only leaves whose name is `*`; a `cfg(test)`-only import is also "unused" in the lib build and must stay |

After the sweep, `grep -rn 'TODO(house-style)' crates/` returning nothing is the
completion signal, and `cargo check --all-targets` - not the lib-only build - is
the gate that proves it.

## Ordering that matters

Restore from a snapshot and re-run the whole pipeline rather than layering a
second transform on a half-transformed tree: the passes read the tree they are
given, and a wrong pass 3 leaves path fragments (`crate::Struct::Struct`) that
pass 4 will happily treat as ordinary text.

Take a `tar czf` snapshot of both `Source/` trees before the first `--apply`
(the tree is dirty by design - an external auto-committer sweeps it, so `git
checkout` is not a recovery path).
