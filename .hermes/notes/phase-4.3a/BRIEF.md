# Phase 4.3a brief - `if`/`for`/`while` becomes `match`/combinators

Repo: `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`
Rule source: `.hermes/notes/HOUSE-STYLE-REFACTOR.md` §4.3 (branching/iteration) and
§4.7 (vertical rhythm). Read both sections before you start.

Scope: ONLY the `.rs` files under `crates/aphrodite/Source/` inside the module
directories your own dispatch names. Nothing else. Do not open, read or edit a
file outside your module list - a sibling worker owns it and is editing it right
now.

This is a **style** refactor. Output behaviour must be identical: same values,
same order of evaluation, same short-circuit order, same borrow scopes, same
early exits.

## 1. The catalogue

Every conditional branch becomes a `match` on the decision value. Every loop
becomes a combinator or a `while let` drain.

### Boolean condition, statement position

```rust
// BEFORE
if Cond {
	Body;
}
```

```rust
// AFTER
match Cond {
	true => {
		Body;
	}
	false => {}
}
```

Two branches:

```rust
// BEFORE
if Cond {
	Yes;
} else {
	No;
}
```

```rust
// AFTER
match Cond {
	true => {
		Yes;
	}
	false => {
		No;
	}
}
```

### `if` / `else if` / `else` chain - guards keep the short-circuit order

```rust
// BEFORE
if A {
	One;
} else if B {
	Two;
} else {
	Three;
}
```

```rust
// AFTER
match () {
	_ if A => {
		One;
	}
	_ if B => {
		Two;
	}
	_ => {
		Three;
	}
}
```

The guard form is the ONLY correct form for a chain: `match (A, B)` would
evaluate `B` even when `A` is true, which is a behaviour change (side effects,
panics, cost). Never build a tuple of the conditions.

### `if let`

```rust
// BEFORE                              // AFTER
if let Some(X) = E { Body; }           match E { Some(X) => { Body; } None => {} }
if let Ok(X) = E { Body; }             match E { Ok(X) => { Body; } Err(_) => {} }
if let Err(E) = R { Body; }            match R { Err(E) => { Body; } Ok(_) => {} }
if let Some(X) = E { A } else { B }    match E { Some(X) => { A } None => { B } }
```

Patterns that carry a refutable payload keep working the same way in a `match`
arm - `Some(X)` binds `X` exactly as `if let` did.

### `for` loops

```rust
// BEFORE
for Item in Items {
	Body(Item);
}
```

```rust
// AFTER
Items.for_each(|Item| {
	Body(Item);
});
```

- `continue` in the body -> `.filter(|X| !SkipCond(X)).for_each(...)`, or an
  early `return` from the closure when the skip condition is a plain boolean.
- `break` in the body -> `.take_while(|X| !StopCond(X)).for_each(...)` when the
  stop condition does not borrow what the body mutates, otherwise
  `.try_for_each(|X| -> Result<(), ()> { ... })` with `Err(())` as the break
  sentinel and the result discarded (`let _ = ...;`).
- `for` that only maps one collection into another ->
  `.map(...).collect()` (use `.filter_map(...)` when the body's `if let` was
  the filter).
- `for` that accumulates into a `&mut` local - `.for_each` with the closure
  capturing it, or `.fold(...)` when the accumulator is the loop's whole state.
- A `for` whose body mutates the collection being iterated cannot become a
  combinator (borrow conflict) - report it as an exception.

### `while`

```rust
// BEFORE
while Cond {
	Body;
}
```

```rust
// AFTER
loop {
	match Cond {
		true => {
			Body;
		}
		false => break,
	}
}
```

`while let Some(X) = Stack.pop() { ... }` is an ALLOWED final form (the card
names it as the drain shape) - keep it, do not rewrite it into a combinator.

`loop { ... break Value; }` -> `match` on the terminal condition, as above, with
the value produced by the arm.

### Statement position vs expression position

A `match` used as a value (`let X = match C { true => A, false => B };`) keeps
its arms bare. A `match` in statement position takes block arms and a `false`/
`None`/`Err` arm of `{}`. rustfmt is configured with `match_arm_blocks = true`,
so every arm body that is not a single expression is a block.

## 2. Behaviour hazards - each of these is a real bug if you miss it

1. **Temporary lifetime.** `if Guard::lock().x > 0 { ... }` drops the guard
   before the body. `match Guard::lock().x > 0 { ... }` keeps the scrutinee
   temporary alive until the end of the whole `match`, so a body that locks the
   same mutex deadlocks. Bind the scrutinee to a local first
   (`let Cond = Guard::lock().x > 0;`) whenever the condition produces a
   temporary that owns a lock, guard, file handle or borrow.
2. **Short-circuit order.** `A && B` evaluates `B` only when `A` is true. Never
   turn a chain into a tuple or into a value that forces `B`. Use guards.
3. **Borrow scope.** A `match` moves its scrutinee exactly where the old `if`
   borrowed it -> `E0505`/`E0382`. Match the same way the `if` did
   (`match &E` / `match E` / `match E.as_ref()`), and if no spelling keeps the
   old borrows, report the site as an exception instead of cloning.
4. **`?` and early return.** A combinator closure returns from the CLOSURE, not
   from the enclosing function. Use `.try_for_each(|X| -> Result<()> { ... })?;`
   when the body used `?`, or keep the loop and report it.
5. **`return` inside a converted loop body** used to leave the function; inside
   `.for_each` it leaves only the closure. `.try_for_each` + `?` preserves the
   original exit.
6. **`break` out of a nested loop / labelled `break`** cannot be expressed by a
   combinator - exception.

## 3. Allowed, and only these

- `if` as a `match` guard (`_ if Cond => ...`).
- `#[cfg(...)]`, `cfg!()`, and `if` inside a `const`/`static` initialiser.
- `while let Some(X) = ...pop() { ... }` - the drain form.
- A site a `match` cannot express without making the code worse. Each one goes
  in your report with `file:line` and the reason. Do not leave an `if` behind
  silently.

## 4. What you must not touch

- `pub use` / re-exports, identifiers, `anyhow`/`thiserror`, logging, comments'
  content, `#[serde(...)]` attributes, module structure, `Cargo.toml`.
- Nothing outside your module list. A sibling worker is editing those files.
- **No `git` command at all** (no add, no commit, no checkout, no stash). An
  external auto-committer sweeps the tree; a clean `git status` does not mean
  you did not write.
- **No bulk rewrite tooling**: no `sed`, no `awk`, no `perl -i`, no
  `python -c` rewriting files, no heredoc appends. Edit through the `patch`
  and `write_file` tools only. This is a hard rule: those tools silently
  corrupt Rust files.

## 5. Verification

From the repo root, after each module you finish (not after every file):

```sh
cargo check -p aphrodite --all-targets > .hermes/tmp/check- < your-bucket > .log 2>&1
echo "EXIT:$?"
```

Read the log. Errors that point at files **outside your module list** are a
sibling worker's in-flight edit, not yours: wait about 60 seconds, re-run, and
never edit those files. Errors that point at your own files are yours - fix
them before moving on.

Do NOT run `cargo fmt`, `cargo clippy`, or `cargo test`: the parent runs those
once at the end (a sibling's half-converted tree would poison them). Write code
in the repo's shape yourself: hard tabs, max width 120, trailing comma in a
vertical arm list, block arms.

Source `~/.envsh` first if `cargo` is not on PATH.

## 6. Report format

Return exactly this shape:

```
BUCKET: <your bucket letter>
CONVERTED: <number of if/for/while sites rewritten>
MODULES: <module dir -> files touched -> sites converted, one line each>
EXCEPTIONS:
- <file>:<line> - <kind: if|if let|for|while> - <why a match/combinator cannot express it>
CHECK: <exit code of the last cargo check>
```

`CONVERTED` must equal the number of sites you actually changed, not the number
of sites that existed.
