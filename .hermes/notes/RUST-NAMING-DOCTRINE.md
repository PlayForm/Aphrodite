# One word per item - the naming doctrine

The repository now separates every item into its own leaf, under a PascalCase directory chain that
names the domain. That changes what an identifier has to carry.

`Preview::State::Struct::PreviewMaximum` says _preview_ three times: once in the path, once in the
module, once in the field. The field only has to say **which** thing it is, because the path already
says **what kind** of thing and **where** it lives.

```rust
// Before
pub struct Struct {
	pub PreviewMaximum:Option<u32>,
	pub MarkerFormat:Option<String>,
	pub MarkerHint:Option<String>,
	pub MarkerHintEnabled:bool,
	pub Templates:Option<TemplatesConfiguration>,
	pub Family:Option<String>,
}

// After - crates/aphrodite/Source/Preview/State/Struct.rs
pub struct Struct {
	/// `[previews] preview_max_chars`; `None` = unlimited.
	pub Limit:Option<u32>,
	/// `[templates.marker] format`; `None` = the canonical fallback grammar.
	pub Format:Option<String>,
	/// `[templates.marker] hint`; `None` = no hint line.
	pub Hint:Option<String>,
	/// `[prompts] ccr_marker_hint`.
	pub Enable:bool,
	/// `[templates]`; `None` = the hardcoded per-arm builders stay in force.
	pub Template:Option<Configuration>,
	/// `[previews] model_family`; `None` = the renderer's default family.
	pub Family:Option<String>,
}
```

## 1. The rule

> **The path is the namespace. The name is the word.**

A reader reaches an item by its full path (`Preview::State::Struct`, `Proxy::Threshold::Resolve::Fn`),
so the identifier inside carries only the distinction that the path does not already make.

This is **not** abbreviation. `Env`, `Cfg`, `Impl`, `Num` stay forbidden - they shorten a word that
was never redundant. `Limit` is not a shortening of `PreviewMaximum`; it is the one word left once
`Preview` is carried by the path.

### One word, not a compound

An identifier is **a single capitalised token** - not two or three glued together.
`PreviewMaximumChars` is three words wearing one name; `Limit` is one.

The words you remove do not disappear: they move **into the path**, which is the namespace. That is
what keeps the rename lossless.

| Compound today           | Where the extra words go                         | One-word name |
| ------------------------ | ------------------------------------------------ | ------------- |
| `PreviewMaximumChars`    | path `Preview::State`                            | `Limit`       |
| `MarkerHintEnabled`      | path `Template::Marker`                          | `Enable`      |
| `CodeStructureMap`       | path `Configuration::Proxy::Previews::Structure` | `Map`         |
| `RustPreviewLines`       | path `Configuration::Proxy::Previews::Rust`      | `Lines`       |
| `CacheCompressThreshold` | path `Proxy::Threshold::Cache::Compress`         | `Value`       |
| `TemplatesConfiguration` | path `Template::Configuration`                   | `Struct`      |
| `MarkerFormat`           | path `Template::Marker`                          | `Format`      |

A single token that happens to be a compound word in English **stays**: `Threshold`, `Workspace`,
`Envelope`, `Payload`, `Hostname`, `Upstream`. The rule forbids gluing two capitalised words, not
compound etymology.

When no single word carries the distinction - a domain term that only exists as a pair - keep two
tokens, state the reason in the doc comment, and add the name to the declared allowlist (§7) so the
sweep can tell a decision from an oversight.

Two things this does **not** reach: standard-library and dependency names (`NonZeroUsize`,
`CString`, `Cow`) and external contract names (serde keys, CLI flags, ABI symbols), which are
snake_case or kebab by their own rules.

## 2. Where it applies

| Surface                  | What the name carries                                                   | Example                                              |
| ------------------------ | ----------------------------------------------------------------------- | ---------------------------------------------------- |
| **Fields**               | which value, inside a struct whose path names the object                | `Limit`, `Format`, `Hint`, `Enable`, `Template`      |
| **Arguments and locals** | which value, inside a function whose path names the operation           | `Fn(Source:&str, Limit:NonZeroUsize)`                |
| **Functions**            | the verb; the directory names the object it acts on                     | `Threshold::Resolve::Fn`, `Preview::Render::Cap::Fn` |
| **Structs and enums**    | the kind - the leaf exports `Struct` / `Enum`, the path says what it is | `Preview::State::Struct`                             |
| **Traits**               | the kind - and only where more than one implementation exists           | `Marker::Format::Trait`                              |

A struct that exists to carry one operation's inputs is `Input::Struct` in that operation's
directory, not `TerminalCompressionInput`.

## 3. Traits become structs by default

The research corpus says it plainly: _do not introduce traits where one concrete implementation is
sufficient._ A trait with a single implementation is a struct that has not admitted it.

- One implementation → delete the trait, make the type the struct, name the leaf `Struct`.
- Genuinely several implementations → keep the trait, name the leaf `Trait`, and let each
  implementation own its own `Struct` leaf under it.
- A trait existing only to break a dependency cycle is a boundary decision, not a naming one - say so
  in the doc comment rather than leaving it to look like polymorphism.

## 4. The collision rule - the one thing that limits the doctrine

A word must be **unique within its scope**. `Limit` is right in a struct about previews; it is wrong
in a function that also handles a body limit and a cache limit:

```rust
// Wrong - three Limits, no reader can tell them apart
fn Fn(Limit:NonZeroUsize, Limit:NonZeroUsize, Limit:NonZeroUsize)

// Right - the qualifier returns only where the scope makes the bare word ambiguous
fn Fn(PreviewLimit:NonZeroUsize, BodyLimit:NonZeroUsize, CacheLimit:NonZeroUsize)
```

So the procedure is: **strip the redundant qualifier, then re-check uniqueness in scope.** If two
names collapse to the same word in one scope, one of them keeps its qualifier - and the qualifier
that survives is the one naming the _distinction_, not the domain.

## 5. What must not be renamed, and how to rename around it

Three surfaces have a second name, and renaming the identifier silently renames the contract. Every
one of these needs the external name pinned explicitly.

| Surface                                                                           | The risk                                                  | The pin                                      |
| --------------------------------------------------------------------------------- | --------------------------------------------------------- | -------------------------------------------- |
| **serde keys** - anything serialised from `aphrodite.toml` or onto the proxy wire | renaming the field changes the config key or the JSON key | `#[serde(rename = "preview_max_chars")]`     |
| **CLI flags** - a clap field derives `--preview-max-chars`                        | renaming ships `--limit` to every script                  | `#[arg(long = "preview-max-chars")]`         |
| **ABI symbols** - `pub extern "C" fn aphrodite_*`                                 | the symbol _is_ the contract                              | leave the symbol; rename only internal items |

The worked example above is exactly this case: the field becomes `Limit` while the key stays
`preview_max_chars`, and the doc comment names the key so a reader can find it without the source.

**A rename that changes a config key is a breaking change, not a rename.** If the pin is missing, add
it in the same edit - and verify the emitted key, not the source.

## 6. The doc comment is not optional

A one-word name is only unambiguous because its doc comment names the external contract:

```rust
/// `[previews] preview_max_chars`; `None` = unlimited.
pub Limit:Option<u32>,
```

Every renamed item carries a line saying **what external thing it maps to** and **what `None` / `0` /
empty means**. Without that line the rename has removed information instead of relocating it.

## 7. What this doctrine is not

- **Not a shortening pass.** `Configuration` does not become `Config`; `Environment` does not become
  `Env`. The word stays whole; only the redundant qualifier goes.
- **Not a licence to lose the domain.** If the path is `Proxy::Threshold::Struct`, `Value` is a poor
  name - it says nothing even with the path. Prefer the noun the path implies but does not spell:
  `Compress`, `AutoExpand`, `Inline`.
- **Not applied to wire data.** A struct whose fields are a published JSON schema keeps its field
  names; only its _local_ bindings and its internal accessors get the treatment.
- **Not a bulk `sed`.** Each rename is a leaf-local edit plus its consumers, because the collision
  check in §4 has to be made per scope.

## 8. Procedure

1. **Inventory the leaf.** Read the whole file; list every identifier in it.
2. **Strip the redundant qualifier** the path already carries. `Preview::State` → drop `Preview`.
3. **Check uniqueness in scope** (§4). Qualify only where a collision survives.
4. **Pin every external name** (§5) - serde, clap, ABI - in the same edit.
5. **Repoint consumers** in the same step; the tree stays self-consistent.
6. **Add or fix the doc comment** (§6).
7. **Verify the contract, not the source**: the emitted TOML key, the JSON key, the CLI flag and the
   ABI symbol are each unchanged. A source grep cannot prove this.

## Claim-to-test

| Claim                                                  | Probe                                                                                            | Pass condition                                                                           |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| No item repeats the qualifier its path already carries | read each leaf; the identifier's words are not a prefix of the path                              | no `Preview::State::Struct::PreviewMaximum`                                              |
| Every identifier is one word                           | `grep -rnE '\b[A-Z][a-z0-9]+([A-Z][a-z0-9]+){2,}\b' crates/*/Source --include=*.rs`              | every hit is a dependency/std name, an external contract name, or on the allowlist in §7 |
| The removed words are in the path                      | for each renamed item, name the directory that now carries the dropped qualifier                 | no information lost                                                                      |
| No word collides within one scope                      | `Fn(...)` and `struct Struct { ... }` signatures                                                 | no two parameters or fields share a word                                                 |
| External names are unchanged                           | the emitted `aphrodite.toml` keys, the JSON keys, `--help`, and `Maintain/check_ffi_contract.py` | byte-identical to before the rename                                                      |
| Every renamed item is documented                       | each `pub` field/arg has a `///` naming its external contract                                    | none missing                                                                             |
| No abbreviation was introduced                         | `grep -nE '\b(Env\|Cfg\|Impl\|Num\|Dir\|Msg\|Ctx)\b'`                                            | no hits                                                                                  |
| Traits with one implementation are gone                | `grep -rn '^pub(\(crate\))? trait '`                                                             | each remaining trait has ≥2 implementations                                              |
