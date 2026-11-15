# Aphrodite message catalogue - every human-facing string as data

An `anyhow!` string is a parameter. Today it is a literal welded into the call site:

```rust
CLI.APIKey
	.trim()
	.is_empty()
	.then(|| anyhow::anyhow!("no API key configured - set APHRODITE_API_KEY env var or pass --api-key, or run `aphrodite setup`"))
	.map_or(Ok(()), Err)
```

That message is the entire user-facing diagnosis for the most common startup failure, and the only
way to improve its wording is to edit Rust and rebuild. This document specifies the catalogue that
replaces it, on its own board (`errors`), after the functional rewrite and the parameter surface land.

Measured surface at the audit HEAD:

| Shape                                | Count                                                |
| ------------------------------------ | ---------------------------------------------------- |
| `anyhow!(...)`                       | 29                                                   |
| `bail!(...)`                         | 20                                                   |
| `.context(..)` / `.with_context(..)` | 34                                                   |
| `#[error("...")]` (thiserror)        | 0                                                    |
| `eprintln!`                          | 2                                                    |
| **total message sites**              | **~83** (75 in `aphrodite`, 8 in `aphrodite-hermes`) |

Small enough to catalogue exhaustively, large enough that the same six-word phrase is currently
written four times in three modules.

## 1. The class rule - the boundary that keeps this safe

Not every message may be reworded at runtime. Three classes, and the class is a property of the
**consumer**, not of the text.

| Class          | Consumer                                                                 | Example                                                                                      | Rewordable?                                                                |
| -------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **Diagnostic** | a human reading a log, a CLI failure, or the model reading a tool result | `no API key configured - set APHRODITE_API_KEY ...`                                          | **yes**, hot-reloadable                                                    |
| **Contract**   | a program that matches on the text                                       | the three `CheckContent` refusals, the `{"error": ...}` JSON the C ABI returns to the plugin | **keyed and stable**; the wording may be versioned, never silently changed |
| **Internal**   | nobody; it is a debug breadcrumb                                         | `tracing::debug!` fields                                                                     | not in the catalogue                                                       |

The test for **Contract**: _does any code, test, script or client outside this function depend on
these bytes?_ `Guard::CheckContent`'s three refusals are documented as "the whole diagnosis the C
side gets" - a host marshalling its own buffers distinguishes "sent nothing" from "sent too much"
from "sent a NUL" **by that string**. Rewording it silently breaks a consumer that cannot see the
source.

So a Contract message keeps a stable key _and_ a stable text; it participates in the catalogue for
provenance and discovery, but the overlay refuses to change it unless the caller opts in with an
explicit version bump. A Diagnostic message is free to change between runs.

## 2. Keys are the identity, not the English

```toml
[message.boot.credential.missing_api_key]
class = "diagnostic"
text = "no API key configured - set {Variable} env var or pass --api-key, or run `aphrodite setup`"
fields = { Variable = "String" }

[message.abort.content.empty]
class = "contract"
text = "empty content"

[message.abort.content.too_large]
class = "contract"
text = "content exceeds {Limit} limit"
fields = { Limit = "String" }
```

A key is a stable dotted identifier (`boot.credential.missing_api_key`), never derived from the text.
The text can be reworded, translated or A/B-tested without changing a single call site or breaking a
consumer that matches the key.

## 3. Typed placeholders, checked at compile time

The reason to prefer this over raw `format!` strings is that the compiler can check the fields:

```rust
Error::Raise!(Boot::Credential::MissingAPIKey, Variable = "APHRODITE_API_KEY")
```

The macro - generated from the catalogue - expands to a constructor whose argument list _is_ the
catalogue's `fields` table. Consequences:

- a missing field is a **compile error**, not a runtime `{}`-count panic;
- an unused field is a compile error;
- a key that does not exist is a compile error;
- the text never appears in the source, so it cannot drift from the catalogue.

This is the same discipline §4.3 of the parameter spec applies to scalars, applied to prose.

## 4. One loader, reused

The message catalogue is an **asset**, not a new subsystem: it declares itself through
`Embedded::Declare!` (§4.4 of `APHRODITE-PIPELINE-PARAMETERS.md`) with `parse: Toml`, and inherits
the overlay, the provenance and the hot reload that mechanism already owns.

```
crates/aphrodite/templates/messages.toml     compiled default, include_str!'d via the macro
~/.hermes/aphrodite/assets/messages.toml     user overlay, hot-reloaded
```

- `aphrodite messages list` - every key, its class, and where its text came from.
- `aphrodite messages show boot.credential.missing_api_key` - the resolved text and the layer.
- A reload swaps the catalogue atomically; a **Contract** entry is not swapped unless the overlay
  declares the same `version`.
- A malformed overlay warns with the key and the line, and keeps the compiled text. It never turns a
  bad edit into a confusing startup failure - that would be the error path producing a worse error.

## 5. The `anyhow` shape

`anyhow` carries a context chain, and the catalogue must not flatten it:

```rust
// crates/aphrodite/Source/Error/Raise/Fn.rs
pub(crate) fn Fn(Key:Key, Fields:Fields) -> anyhow::Error
```

- A `bail!("...")` becomes `return Err(Error::Raise!(...))`, or better a `?` on the helper.
- A `.context("...")` keeps its position in the chain: `Error::Context!(Key, Fields)` wraps, it does not
  replace.
- `Display` for a Contract message is the resolved text - byte-identical to today at the default
  catalogue, which is what makes the migration verifiable rather than hopeful.
- No `thiserror` is in use (0 `#[error]` sites), so there is no derive to migrate. If one appears
  later, its `#[error("...")]` routes through the same catalogue.

## 6. The completeness rule

> No string literal in an error position outside the catalogue.

Enforced by a text probe, not by review: the count of `anyhow!`/`bail!`/`.context(` occurrences with
a literal argument must reach zero, except an explicitly listed residue (a `Protocol` message whose
text is part of an ABI, named one by one in the doc).

## 7. Migration phases

| Phase  | Deliverable                                                                                         | Acceptance                                                                                                 |
| ------ | --------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **M1** | Exhaustive inventory: every site classified Diagnostic / Contract / Internal, with its proposed key | the counts in §0 match the tree; every site has a key                                                      |
| **M2** | `messages.toml` + generated key enum + `Error::Raise!` macro                                        | a call site cannot name a key that does not exist, and cannot omit a field                                 |
| **M3** | Overlay + provenance + hot reload, via the shared asset loader                                      | editing the user overlay changes a Diagnostic message without a rebuild; `messages show` names the layer   |
| **M4** | Migrate all ~83 sites                                                                               | probe from §6 returns only the named residue; every message at defaults is byte-identical to pre-migration |
| **M5** | Wording benchmark: two catalogues, one corpus, compare preview/diagnosis fidelity                   | the harness reports a difference rather than a preference                                                  |

M4 is where the risk lives, so its acceptance is byte-identity at defaults: a migration that changes
a single message at the default catalogue has changed behaviour, not wording.

## 8. What this cannot do - the honest boundary

The request behind this epic is broader than messages: _everything reduced to helpers, macros and
code-generation, supporting hot-reload and plug-and-play pipelines_. That is achievable for **data**
and not for **behaviour**, and the distinction is worth stating plainly so the ambition lands
somewhere real:

| Layer                                                | Hot-reloadable? | Why                                                                     |
| ---------------------------------------------------- | --------------- | ----------------------------------------------------------------------- |
| scalars, thresholds, caps (parameter spec)           | **yes**         | read once into an immutable value; a reload swaps it                    |
| vocabularies, prompts, messages (this spec, §4.4)    | **yes**         | same mechanism, different parser                                        |
| pipeline **shape** - which stage runs, in what order | **no**          | it is control flow in compiled code; a reload would mean an interpreter |
| ABI symbols, wire keys, marker grammar               | **never**       | another process or stored data depends on the bytes                     |

A genuinely plug-and-play pipeline would need the stages to be _named and composed_ rather than
called - an interpreter over a stage table. That is a different product with a different performance
profile, and it is not required to get 90% of the value: with parameters, assets and messages all
overlaid, everything an operator would want to tune is tunable, and the only thing requiring a
rebuild is a change to the graph itself.

If that graph-level goal is wanted later, it is its own epic with its own benchmark, and this
document's job is to make sure the three data layers are already uniform when it starts.

## Claim-to-test

| Claim                                        | Probe                                                                          | Pass condition                                                       |
| -------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| Every message site is catalogued             | `grep -rnE 'anyhow!\(\|bail!\(\|\.(with_)?context\(' crates/*/Source`          | each hit resolves to a catalogue key, or is a named Protocol residue |
| No message text remains in source            | the same probe, counting literal-argument forms                                | zero outside the residue list                                        |
| A missing placeholder field does not compile | remove a field from one call site                                              | compile error naming the key and the field                           |
| A Diagnostic message reloads                 | edit the user overlay, re-read                                                 | new text without a rebuild; `messages show` reports layer `user`     |
| A Contract message does not reload silently  | edit a Contract entry in the overlay                                           | refused with a warning naming the missing `version` bump             |
| Defaults are byte-identical after migration  | compare every message at the default catalogue against the pre-migration build | no difference                                                        |
| A bad overlay degrades                       | write invalid TOML to the overlay                                              | one warning with the line; compiled text still used                  |
