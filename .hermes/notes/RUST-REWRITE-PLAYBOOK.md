# Rust rewrite playbook

The style guide (`.hermes/notes/RUST-STYLE-FUNCTIONAL.md`) states the rule; this document states how
one concrete site in this tree is rewritten under it. Every recipe is the same five fields, so a
worker can scan for its own site and stop at the one it owns.

- **When** - the shape that calls for the rewrite, in one sentence.
- **Site** - `crates/...` path and line, taken from the measured inventory
  (`.hermes/notes/RUST-FUNCTIONAL-AUDIT.md`). Omitted only when the recipe is general.
- **Before** - the current code, trimmed to the decision, copied from the tree at the line named.
- **After** - the rewrite under `.hermes/notes/HOUSE-STYLE-REFACTOR.md` §4.1/§4.2/§4.7: PascalCase
  identifiers, one export per leaf file, the item named by its kind, the whole module path at the
  call site, tabs, width 120.
- **Preserved** - what must not change: arm order, evaluation order, ownership, lock lifetime, error
  text, exit path.
- **Probe** - the command or reading that settles it. A green build proves the types line up and
  nothing else, so it is never the whole answer.

A recipe marked **BEHAVIOUR** changes what the program does, not only how it is spelled. Those are
defect fixes or stricter parses, they are named as such, and they never ride in the same change as a
style-only pass - a reviewer must be able to read one and say whether the difference is intended.

Two standing constraints apply to every recipe and are not repeated in each one. A new helper is a
leaf module whose directory carries the domain, whose file carries the kind and whose export is named
by that kind; the call site writes the whole path, because §4.2 forbids the re-export that would
create a second name. And nothing here reaches the C ABI's exported symbols, the `serde` field names
on the wire types, or the spellings of a third-party API - see the style guide's §9.

## Group 1 - predicates, absence, composition

### 1.1 Ordered rejection rules stay guards

**When** - a validator whose every arm is `_ if <predicate>` over a unit scrutinee, so the match is a
policy list wearing a match's clothes.

**Site** - `crates/aphrodite/Source/ABI/Guard.rs:94`

**Before** -

```rust
pub(crate) fn CheckContent(Content:&str) -> anyhow::Result<()> {
	match () {
		_ if Content.is_empty() => anyhow::bail!("empty content"),
		_ if Content.len() > MaximumContent => anyhow::bail!("content exceeds 16MB limit"),
		_ if Content.contains('\0') => anyhow::bail!("content contains null bytes"),
		_ => Ok(()),
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Guard.rs
pub(crate) fn CheckContent(Content:&str) -> anyhow::Result<()> {
	if Content.is_empty() {
		anyhow::bail!("empty content");
	}

	if Content.len() > MaximumContent {
		anyhow::bail!("content exceeds 16MB limit");
	}

	if Content.contains('\0') {
		anyhow::bail!("content contains null bytes");
	}

	Ok(())
}
```

**Preserved** - the three predicates are tested in the same order and produce the same three strings,
so a payload that is both empty and over the cap still reports `empty content`.

**Probe** - `awk 'NR>=94&&NR<=104' crates/aphrodite/Source/ABI/Guard.rs` prints three `if` blocks and
no `match`; `grep -c 'content exceeds 16MB limit' crates/aphrodite/Source/ABI/Guard.rs` is unchanged
at 1.

### 1.2 A validator lifted into `Reject::Fn` and `?`

**When** - the same ordered-rejection shape appears in a second validator, so the boolean decision
should exist once instead of once per validator.

**Site** - `crates/aphrodite/Source/ABI/Guard.rs:94`

**Before** - as 1.1: three guarded arms, each constructing its own error at the point of rejection.

**After** -

```rust
// crates/aphrodite/Source/Guard/Reject/Fn.rs - declared `pub mod Reject;` in Guard/mod.rs
pub(crate) fn Fn<E>(Reject:bool, Error:impl FnOnce() -> E) -> Result<(), E> {
	if Reject { Err(Error()) } else { Ok(()) }
}
```

```rust
// crates/aphrodite/Source/ABI/Guard.rs - crate-rooted path: this module is also named Guard
pub(crate) fn CheckContent(Content:&str) -> anyhow::Result<()> {
	crate::Guard::Reject::Fn::Fn(Content.is_empty(), || anyhow::anyhow!("empty content"))?;
	crate::Guard::Reject::Fn::Fn(Content.len() > MaximumContent, || {
		anyhow::anyhow!("content exceeds 16MB limit")
	})?;
	crate::Guard::Reject::Fn::Fn(Content.contains('\0'), || anyhow::anyhow!("content contains null bytes"))?;

	Ok(())
}
```

**Preserved** - `?` short-circuits in argument order, so the first failing predicate still wins, and
the error closure runs only for the predicate that rejected - the messages are built lazily, exactly
as the guard arms built them lazily.

**Probe** - add a fourth caller in another validator, then `cargo check -p aphrodite`; the helper
must be reachable without a `pub use` (`grep -rn 'pub use' crates/aphrodite/Source` still prints
nothing). Read the three `?` lines in order against the three `bail!` lines it replaced.

### 1.3 Nullable pointer becomes `Option<NonNull<_>>`

**When** - a C string pointer is checked for null and the null case returns early, so the pointer
itself is the value worth interpreting.

**Site** - `crates/aphrodite/Source/ABI/Guard.rs:74`

**Before** -

```rust
pub(crate) unsafe fn Cstr(Ptr:*const c_char) -> Option<String> {
	match Ptr.is_null() {
		true => return None,
		false => (),
	}

	// SAFETY: `Ptr` is non-null here, and per the safety contract of this
	// function it points to a valid, NUL-terminated C string that remains
	// valid for the duration of this call.
	Some(unsafe { CStr::from_ptr(Ptr) }.to_string_lossy().into_owned())
}
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Guard.rs
pub(crate) unsafe fn Cstr(Ptr:*const c_char) -> Option<String> {
	// SAFETY: `Ptr`, if non-null, points to a valid, NUL-terminated C string that stays
	// valid for the duration of this call (the caller's `CStr::from_ptr` contract).
	let Pointer = unsafe { NonNull::new(Ptr as *mut c_char) }?;

	Some(unsafe { CStr::from_ptr(Pointer.as_ptr()) }.to_string_lossy().into_owned())
}
```

**Preserved** - the null check happens before any dereference, the `unsafe` block keeps its contract
comment, and the non-null path produces byte-identical output: `to_string_lossy().into_owned()` is
untouched. `NonNull` proves nothing about the pointee, which is why the `unsafe` block does not
disappear.

**Probe** - `cargo test -p aphrodite` covers `Cstr(null)` and `Cstr(valid)`; additionally
`grep -n 'is_null' crates/aphrodite/Source/ABI/Guard.rs` must no longer match in this function, so
the null branch cannot have been left behind as well as converted.

### 1.4 Eager alternative tuple becomes an `or_else`/`find_map` chain

**When** - a `match` scrutinee is a tuple that computes several alternatives before any arm selects
one, so the alternatives that lose did their work for nothing.

**Site** - `crates/aphrodite/Source/Preview/Text/FirstMeaningful.rs:24`

**Before** -

```rust
Content.lines().map(str::trim).filter(|t| !t.is_empty()).find_map(|t| {
	let core = t.trim_end_matches([',', ';']).trim();
	let mut It = core.chars();

	match (It.next(), It.next()) {
		(Some('{' | '}' | '[' | ']' | '(' | ')'), None) => None,
		_ => Some(t.to_string()),
	}
})
```

**After** -

```rust
// crates/aphrodite/Source/Preview/Text/FirstMeaningful.rs
Content
	.lines()
	.map(str::trim)
	.filter(|Line| !Line.is_empty())
	.find_map(|Line| {
		let Core = Line.trim_end_matches([',', ';']).trim();
		let mut Characters = Core.chars();
		let Structural = matches!(Characters.next(), Some('{' | '}' | '[' | ']' | '(' | ')'));
		let Trailing = Characters.next().is_none();

		(!(Structural && Trailing)).then(|| Line.to_string())
	})
```

**Preserved** - the same line is tested, the same two characters are consumed from the same trimmed
form, and `find_map` still stops at the first line that is not structural noise. `then` is lazy, so
`Line.to_string()` allocates only for the line that is actually returned.

**Probe** - the module's own test feeds a pretty-printed body and a bare-brace body; assert the same
two outputs before and after. Then `grep -c 'match (' crates/aphrodite/Source/Preview/Text/FirstMeaningful.rs`
must drop to 0.

### 1.5 A cached value resolved through `filter` then `map`

**When** - a cache read inspects an entry for staleness, mutates on the stale branch and returns on
the fresh one, so the absence and the decision are tangled in one `match`.

**Site** - `crates/aphrodite/Source/Proxy/Cache/Get/Fn.rs:21`

**Before** -

```rust
pub(crate) fn Fn(State:&AppState, Ck:u64) -> Option<Vec<u8>> {
	State.ResponseCache.lock().ok().and_then(|mut Cache| {
		let Expired = Cache
			.peek(&Ck)
			.map(|(InsertedAt, _)| InsertedAt.elapsed() >= State.ResponseCacheTtl)
			.unwrap_or(false);
		match Expired {
			true => {
				Cache.pop(&Ck);

				None
			},
			false => Cache.get(&Ck).map(|(_, body)| body.clone()),
		}
	})
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Cache/Get/Fn.rs
pub(crate) fn Fn(State:&AppState, Ck:u64) -> Option<Vec<u8>> {
	State.ResponseCache.lock().ok().and_then(|mut Cache| {
		// `peek` does not promote recency; only `get` does, so the LRU order still reflects
		// served entries and never inspected-and-dropped ones.
		Cache
			.peek(&Ck)
			.filter(|(InsertedAt, _)| InsertedAt.elapsed() >= State.ResponseCacheTtl)
			.map(|_| Cache.pop(&Ck))
			.unwrap_or_else(|| Cache.get(&Ck).map(|(_, Body)| Body.clone()))
	})
}
```

**Preserved** - `peek` still runs before `get` and still does not promote; the stale branch still
pops before returning `None`; the fresh branch still clones the stored bytes, so a replayed response
stays byte-identical. The expiry comparison and the TTL source are unchanged.

**Probe** - read the two cache calls in the new order against the old: `peek` → conditional `pop` →
conditional `get`. `cargo test -p aphrodite` exercises the TTL boundary; assert that a fresh entry is
still promoted (`peek` returns the same insert instant twice across two calls).

### 1.6 `map_err(...)?` instead of a match with a continuing `Ok` arm

**When** - a fallible call is matched only to build the rejection, and the success arm exists to fall
through to the next statement.

**Site** - `crates/aphrodite/Source/Guard/Bearer/Middleware.rs:40`

**Before** -

```rust
match CheckBearerToken(Token.as_deref(), Auth) {
	Err(Message) => {
		return Err((
			StatusCode::UNAUTHORIZED,
			Json(serde_json::json!({"error": format!("{Message:#}")})),
		));
	},
	Ok(()) => (),
}
```

**After** -

```rust
// crates/aphrodite/Source/Guard/Bearer/Middleware.rs
CheckBearerToken(Token.as_deref(), Auth).map_err(|Message| {
	(
		StatusCode::UNAUTHORIZED,
		Json(serde_json::json!({"error": format!("{Message:#}")})),
	)
})?;
```

**Preserved** - the status code, the JSON shape and `format!("{Message:#}")` are character-identical,
so a client sees the same 401 body. The conversion runs only on the error path, as the arm did.

**Probe** - `cargo test -p aphrodite` plus a `curl -s -o /dev/null -w '%{http_code}'` against a
running proxy with a wrong bearer token: still 401. Diff the two `json!` literals by eye - they must
be the same string.

### 1.7 A `HashMap` built by `collect` instead of mutated

**When** - a map is declared mutable, filled by repeated `insert`, and never read until it is
returned, so the mutation is construction.

**Site** - `crates/aphrodite-hermes/Source/Tool/Registry/Fn.rs:39`

**Before** -

```rust
pub(crate) fn Fn() -> HashMap<&'static str, ToolHandler> {
	let mut m:HashMap<&'static str, ToolHandler> = HashMap::new();
	m.insert("aphrodite_catalog", Catalog::Fn::Fn);
	m.insert("aphrodite_compress", Compress::Fn::Fn);
	// ... one line per family, `#[cfg(debug_assertions)]` gated for `Debug`
	m
}
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Tool/Registry/Fn.rs
pub(crate) fn Fn() -> HashMap<&'static str, ToolHandler> {
	// Every family contributes exactly one pair; a new tool is a new leaf plus one row here.
	let Pairs = [
		("aphrodite_catalog", Catalog::Fn::Fn),
		("aphrodite_compress", Compress::Fn::Fn),
		// ... one row per family
	];

	Pairs.into_iter().collect()
}
```

**Preserved** - the map has the same keys, the same handlers and the same capacity behaviour (both
build one allocation and grow once); the `#[cfg(debug_assertions)]` gate still decides whether the
debug family is in the array at all. Duplicate keys cannot appear in an array literal, which is a
stronger statement than `insert` overwriting silently.

**Probe** - `grep -c 'm.insert' crates/aphrodite-hermes/Source/Tool/Registry/Fn.rs` must drop to 0,
and the schema/registry consistency test (`cargo test -p aphrodite-hermes`) must still find exactly
13 declared tools.

### 1.8 `warnings` consumed at a merge boundary, not cloned

**When** - two warning lists are merged by iterating one of them by reference and cloning every
element into the other, so the source survives only to be dropped.

**Site** - `crates/aphrodite-hermes/Source/Directive/Into/Fn.rs:35`

**Before** -

```rust
let (dir, warnings) = Resolve::Fn::Fn(home_param);
warnings.iter().for_each(|w| crate::Debug::Warn::Fn::Fn(w));

let mut result = Materialize::Fn::Fn(&dir);

result
	.get_mut("warnings")
	.and_then(|v| v.as_array_mut())
	.iter_mut()
	.for_each(|arr| arr.extend(warnings.iter().map(|w| serde_json::Value::String(w.clone()))));
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Directive/Into/Fn.rs
let (Directory, Warnings) = Resolve::Fn::Fn(home_param);
Warnings.iter().for_each(|Warning| crate::Debug::Warn::Fn::Fn(Warning));

let mut Result = Materialize::Fn::Fn(&Directory);

// `Option::iter_mut` is the zero-or-one loop: a report with no `warnings` array takes no entries.
Result
	.get_mut("warnings")
	.and_then(|Value| Value.as_array_mut())
	.iter_mut()
	.for_each(|Array| Array.extend(Warnings.into_iter().map(serde_json::Value::String)));
```

**Preserved** - every warning is logged before the merge, exactly once and in the same order, and the
JSON array receives the same strings in the same order. The change is that the strings move instead
of being copied, so the borrow of `warnings` by the log loop must end before the merge - which the
`for_each` already does.

**Probe** - `grep -c 'warnings.iter().map(|w| serde_json::Value::String(w.clone()))'` must drop to 0
and `grep -c 'Warnings.into_iter()'` must be 1. Then read the two statements in order: log loop, then
merge, never the reverse - the log must see the warning even when the array is absent.

## Group 2 - traversal and termination

### 2.1 A bounded parent walk becomes `successors(...).take(17)`

**When** - a depth-capped traversal of a parent map pushes into a `Vec` from inside a
`try_for_each`, so the depth cap is expressed as a fake error.

**Site** - `crates/aphrodite-hermes/Source/Debug/Chain/Fn.rs:38`

**Before** -

```rust
let mut chain = vec![session.to_string()];
let mut cur = session.to_string();

let _ = (0..16).try_for_each(|_| {
	match map.get(&cur) {
		Some(p) if p != &cur => {
			chain.push(p.clone());
			cur = p.clone();

			Ok::<(), ()>(())
		},
		// ... `Err(())` for "no parent"
	}
});
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Debug/Chain/In/Fn.rs - the pure half, no lock
pub(crate) fn Fn(Map:&HashMap<String, String>, Session:&str) -> Vec<String> {
	std::iter::successors(Some(Session.to_owned()), |Current| {
		Map.get(Current).filter(|Parent| Parent.as_str() != Current.as_str()).cloned()
	})
	.take(17)
	.collect()
}
```

**Preserved** - the self-reference guard (`Parent != Current`) still ends the walk, the ceiling is
still 16 hops plus the leaf, and the chain is still leaf-first. The walk still allocates one owned
string per hop because the caller's map is process-global and outlives the chain.

**Probe** - `grep -cE 'Err\(\(\)\)|Ok::<\(\), \(\)>' crates/aphrodite-hermes/Source/Debug/Chain/Fn.rs`
must be 0; then a five-edge chain and a cyclic two-edge chain must both terminate and produce the
same vector as before, which the `Debug::Chain` test asserts.

### 2.2 Budget exhaustion becomes `ControlFlow::Break`

**When** - a scan stops because it spent its budget, and the stop is spelled `return Err(())` inside a
`try_for_each` whose result is discarded.

**Site** - `crates/aphrodite/Source/StructExtract/Rust.rs:32`

**Before** -

```rust
let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
	match *Budget <= 0 {
		true => {
			return Err(());
		},
		false => {},
	}
	// ... classify and push, `*Budget -= Slen as isize + 1`
	Ok(())
});
```

**After** -

```rust
// crates/aphrodite/Source/StructExtract/Rust/Fn.rs
Content.lines().try_for_each(|Line| {
	if *Budget <= 0 {
		return ControlFlow::Break(());
	}

	// ... classify and push, `*Budget -= SignatureLength as isize + 1`

	ControlFlow::Continue(())
})?;
```

**Preserved** - the scan stops at the same line: the budget test is still the first statement of the
iteration, so the line that spends the last unit is still processed and the next one is still
skipped. The budget arithmetic is unchanged, and no line is re-ordered.

**Probe** - `grep -c 'Err(())' crates/aphrodite/Source/StructExtract/Rust.rs` must drop to 0. Then
feed a file with more declarations than the budget and compare the produced signature list against
the pre-change output - same entries, same order, same truncation.

### 2.3 Bounded async accumulation becomes a fallible fold

**When** - an `async` loop accumulates results one `.await` at a time and stops early on the first
failure, so the loop is a fold with a short circuit.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs:47`

**Before** -

```rust
for Attempt in 1..=3u32 {
	// ... build the request
	match Request.body(BodyBytes.clone()).send().await {
		Ok(r) => {
			UpstreamResult = Ok(r);
			break;
		},
		Err(e) => {
			match Attempt < 3 && e.is_connect() {
				true => { /* backoff, sleep, continue */ },
				false => {
					FinalErrorWasTimeout = e.is_timeout();
					UpstreamResult =
						Err(anyhow::Error::new(e).context(format!("send the upstream request to {url}")));
					break;
				},
			}
		},
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Attempt/Fn.rs
futures::stream::iter(1..=3u32)
	.try_fold(None, |_Previous, Attempt| async move {
		let Request = Build::Fn::Fn(&State, Method, url, Headers, Attempt);
		match Request.body(BodyBytes.clone()).send().await {
			Ok(Response) => Ok(Some(Ok(Response))),
			Err(Error) if Attempt < 3 && Error.is_connect() => {
				// backoff, sleep, keep folding
				Ok(None)
			},
			Err(Error) => Ok(Some(Err(anyhow::Error::new(Error).context(format!(
				"send the upstream request to {url}"
			))))),
		}
	})
	.await
	.map(Option::flatten)
```

**Preserved** - at most three attempts, the connect-only retry condition is the same expression, the
backoff is the same `100 * 2^(Attempt-1)` with the same jitter, and the terminal error keeps the same
`context` string. The timeout flag still comes from the error that ended the attempt sequence.

**Probe** - count attempts under a forced connect failure (`grep -c 'upstream retry after connect
error'` in a captured log is at most 2), and confirm a post-send timeout still stops after the first
attempt. The failure path is the one to read: a fold that returns `Ok(None)` on the last attempt
would retry forever, so the guard `Attempt < 3` must be inside the match arm.

### 2.4 A debounce drain becomes `from_fn(|| Rx.try_recv().ok())`

**When** - events accumulated during a debounce window are discarded by a `loop` that breaks on a
boolean, so the drain is a statement block where an iterator would say what it means.

**Site** - `crates/aphrodite/Source/Boot/Listen/Watcher.rs:80`

**Before** -

```rust
loop {
	match Rx.try_recv().is_ok() {
		true => {},
		false => break,
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/Boot/Listen/Watcher.rs
std::iter::from_fn(|| Rx.try_recv().ok()).for_each(drop);
```

**Preserved** - the drain still consumes every buffered event and stops at the first `Empty`, so the
same number of sends are consumed and the channel is left in the same state. `try_recv` is not
called one extra time: `from_fn` stops calling the closure after it returns `None`.

**Probe** - read the two loops against each other: one `try_recv` per iteration, stop on the first
failure, value dropped. Then send three events during a debounce window and confirm the reload runs
once - `grep -c 'config reloaded' ` in the captured log must be 1, not 4.

### 2.5 Channel state matched directly, `None => break`

**When** - a receive is reduced to a boolean before it is matched, so channel closure is
indistinguishable from an empty payload and the `false` arm cannot terminate the loop.

**Site** - `crates/aphrodite/Source/Boot/Listen/Watcher.rs:74`

**Before** -

```rust
loop {
	let Received = Rx.recv().await.is_some();
	match Received {
		true => {
			// debounce, reload, apply thresholds
		},
		false => {},
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/Boot/Listen/Watcher.rs
loop {
	match Rx.recv().await {
		Some(()) => {
			// debounce, reload, apply thresholds
		},
		None => break,
	}
}
```

**Preserved** - nothing changes today, and that is a measured claim rather than a comfortable one:
the sender is moved into the notify closure stored in the `Watcher` local of the same async block,
so no sender can be dropped while the loop runs. The audit records the `false` arm as unreachable at
this site (D2, refuted). What changes is the shape that made it unreachable-looking: a closed channel
now ends the loop instead of spinning on a ready future with no suspension point.

**Probe** - `grep -n 'is_some()' crates/aphrodite/Source/Boot/Listen/Watcher.rs` must no longer match
at the receive; then drop the watcher's sender in a scratch copy and confirm the loop exits rather
than burning CPU - a `top` sample of the spawned task's process is the reading.

### 2.6 A `Result`-as-break sentinel deleted

**When** - a traversal carries `Result<(), ()>` only so `Err` can mean "stop", and the result is
discarded at the call site.

**Site** - `crates/aphrodite-hermes/Source/Session/Watcher/Fn.rs:70`

**Before** -

```rust
let _ = rx.iter().try_for_each(|()| {
	// debounce, reload config, apply to every session
	match keep_watching {
		true => Ok(()),
		false => Err(()),
	}
});
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Session/Watcher/Fn.rs
for () in rx.iter() {
	// debounce, reload config, apply to every session
	if !KeepWatching {
		break;
	}
}
```

**Preserved** - one blocking receive per event, the 500 ms settle before the reload, the drain of
events accumulated during it, and the early stop when `auto_reload` turned off in the file. The
reload itself still runs before the stop is evaluated, so the last event is still applied.

**Probe** - `grep -cE 'Err\(\(\)\)|Ok::<\(\), \(\)>' crates/aphrodite-hermes/Source/Session/Watcher/Fn.rs`
must be 0. Then set `auto_reload = false` in a scratch config, touch the file, and confirm exactly
one reload ran and the watcher thread exited (`ps -M` shows one fewer thread).

## Group 3 - domain modelling

### 3.1 Five independent flags become one route enum

**When** - a function's parameters and the first two branches are a set of booleans and options whose
combinations are the actual decision, so most of the combinations are meaningless.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs:30`

**Before** -

```rust
pub(crate) async fn Fn(
	State:&Arc<AppState>,
	Method:&Method,
	path:&str,
	RequestIDShort:&str,
	T0:std::time::Instant,
	Headers:&axum::http::HeaderMap,
	CacheKey:Option<u64>,
	IsChatCompletion:bool,
	UpstreamResult:anyhow::Result<reqwest::Response>,
	FinalErrorWasTimeout:bool,
) -> Response {
	match UpstreamResult {
		Ok(response) => {
			let Status = response.status();
			let StatusCode = Status.as_u16();
			match () {
				_ if StatusCode >= 500 => { /* 5xx counter */ },
				_ if StatusCode >= 400 => { /* 4xx counter */ },
				_ => {},
			}
			// ... `IsSSE(ContentType)` decides forward-vs-buffer, `CacheKey` decides store
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Route/Enum.rs
pub(crate) enum Enum {
	Streaming { StatusCode:u16, Response:reqwest::Response },
	Buffered { StatusCode:u16, Response:reqwest::Response, Cacheable:Option<u64>, IsChatCompletion:bool },
	Failed { Failure:ForwardFailure },
}
```

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Route/Fn.rs
pub(crate) fn Fn(
	UpstreamResult:Result<reqwest::Response, ForwardFailure>,
	ContentType:Option<HeaderValue>,
	CacheKey:Option<u64>,
	IsChatCompletion:bool,
) -> Route::Enum
```

**Preserved** - the 5xx-before-4xx counter order, the SSE-versus-buffered decision and the cache
store condition all still read the same values in the same order. The enum is built after the
upstream result is known, so no response body is consumed earlier than it was.

**Probe** - `grep -c 'IsSSE(' crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs` must be 1,
inside `Route::Fn::Fn`, and the responder must `match` on the enum with no `Option`/`bool` left in
its parameter list (`awk 'NR>=30&&NR<=45'` on the file). Then compare `/metrics` counters before and
after one 500 and one 404.

### 3.2 A `(mode, name)` tuple becomes `ConfigurationSource`

**When** - a routing decision is made by matching a tuple of two optional strings, so the branch is
spelled twice per case and the third case is the catch-all.

**Site** - `crates/aphrodite/Source/Configuration/Proxy/Multi/Resolve/Fn.rs:47`

**Before** -

```rust
let Listen = match (cfg.Mode.as_deref(), cfg.name.as_deref()) {
	(_, Some("cache")) | (Some("cache"), _) => Self::ApplyPortOverride(Listen, "APHRODITE_CACHE_PORT"),
	(_, Some("token")) | (Some("token"), _) => Self::ApplyPortOverride(Listen, "APHRODITE_TOKEN_PORT"),
	_ => Listen,
};
```

**After** -

```rust
// crates/aphrodite/Source/Configuration/Proxy/Multi/Source/Enum.rs
pub(crate) enum Enum {
	Cache,
	Token,
	Standalone,
}
```

```rust
// crates/aphrodite/Source/Configuration/Proxy/Multi/Source/Fn.rs
pub(crate) fn Fn(Mode:Option<&str>, Name:Option<&str>) -> Source::Enum {
	match (Mode, Name) {
		(Some("cache"), _) | (_, Some("cache")) => Source::Enum::Cache,
		(Some("token"), _) | (_, Some("token")) => Source::Enum::Token,
		_ => Source::Enum::Standalone,
	}
}
```

**Preserved** - `mode` still wins over `name` for the same value, and an unrecognised value in either
field still falls through to no port override. The environment variable names and the listen address
are untouched.

**Probe** - the three inputs `("cache", "token")`, `("token", "cache")` and `(None, None)` must
resolve to cache, token and standalone respectively; read the four arms in order against the old two.
`grep -n 'APHRODITE_CACHE_PORT' crates/aphrodite/Source/Configuration/Proxy/Multi/Resolve/Fn.rs`
must still match exactly once.

### 3.3 Absent and empty collapse into `RetrieveSource`

**When** - a request field is an `Option<String>` whose empty string means the same as its absence,
so the endpoint answers one 400 for two different caller mistakes.

**Site** - `crates/aphrodite/Source/Retrieve/Handle/Fn.rs:27`

**Before** -

```rust
let hash = match &Request.hash {
	Some(h) => crate::Marker::Parse::Normalize::NormalizeHash(h).to_string(),
	None => {
		return (
			StatusCode::BAD_REQUEST,
			Json(RetrieveResponse { Found:false, Content:None, Source:"none".into(), error:Some("`hash` required".into()), Truncated:false, Live:None }),
		)
			.into_response();
	},
};
```

**After** -

```rust
// crates/aphrodite/Source/Retrieve/Source/Enum.rs
pub(crate) enum Enum {
	Hash(String),
	Absent,
}
```

```rust
// crates/aphrodite/Source/Retrieve/Source/Fn.rs
pub(crate) fn Fn(Hash:Option<&str>) -> Source::Enum {
	Hash
		.map(crate::Marker::Parse::Normalize::NormalizeHash)
		.map(str::trim)
		.filter(|Normalized| !Normalized.is_empty())
		.map(str::to_owned)
		.map_or(Source::Enum::Absent, Source::Enum::Hash)
}
```

**Preserved** - the normalisation still runs before the emptiness test, so a marker body echoed back
as the hash still resolves, and the rejection is still the same 400 with the same `hash required`
text. What the type adds is that `Hash` can no longer hold an empty string.

**Probe** - three requests: no `hash`, `{"hash":""}` and a full marker body. All three must produce
the same status as before (400, 400, 200); the last must still resolve, which proves the `filter` did
not run before the normaliser.

### 3.4 `Option<key>` plus `Option<response>` becomes `CacheLookup`

**When** - a cache read is two nested `match`es over two options, so "not cacheable" and "cacheable
but absent" are the same `None` at the call site.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Route/Fn.rs:84`

**Before** -

```rust
match CacheKey {
	Some(Ck) => {
		match Lookup::Fn::Fn(&state, Some(Ck), RequestIDShort, &Method, path.path(), T0) {
			Some(Cached) => {
				return Cached;
			},
			None => {},
		}
	},
	None => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Cache/Lookup/Enum.rs
pub(crate) enum Enum {
	Bypassed,
	Miss(u64),
	Hit(Response),
}
```

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Route/Fn.rs
match crate::Proxy::Cache::Lookup::Fn::Fn(&state, CacheKey, RequestIDShort, &Method, path.path(), T0) {
	Cache::Lookup::Enum::Hit(Cached) => return Cached,
	Cache::Lookup::Enum::Miss(Key) | Cache::Lookup::Enum::Bypassed => {
		// forward; `Miss` still carries the key the response will be stored under
	},
}
```

**Preserved** - a bypassed cache still records no hit and no miss, a miss still records its counter,
and a hit still returns before the upstream request is built. `CacheKey` is still computed from the
same body and still `None` for a non-chat-completion path.

**Probe** - `grep -c 'CacheMisses.fetch_add' crates/aphrodite/Source/Proxy/Cache/Lookup/Fn.rs` must be
1 and `/stats` must show a miss counter that is unchanged by a request with no cache key.

### 3.5 `DidCompress` plus a mutated JSON becomes `RewriteOutcome`

**When** - a function returns a `serde_json::Value` it has been mutating, and the caller learns
whether compression happened by reading a `"compressed"` key out of it.

**Site** - `crates/aphrodite/Source/Hook/ToolResult/Inner/Fn.rs:43`

**Before** -

```rust
match Content.is_empty() {
	true => {
		return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
	},
	false => {},
}
// ... five more of the same shape, then the store + marker + auto-expand branch
```

**After** -

```rust
// crates/aphrodite/Source/Hook/ToolResult/Outcome/Enum.rs
pub(crate) enum Enum {
	Bypassed { Reason:BypassReason },
	Rewritten { Marker:MarkerEntry, Payload:serde_json::Value },
}
```

```rust
// crates/aphrodite/Source/Hook/ToolResult/Inner/Fn.rs
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	Meta:&ToolCallMeta,
) -> Outcome::Enum
```

**Preserved** - the gate order is the contract and does not move: telemetry, file tracking, essential
tool, self tool, threshold, live container, store and marker, auto-expand. Every bypass reason string
survives as a `BypassReason` variant, so the JSON the host receives still carries the same `reason`
values.

**Probe** - `grep -c '"compressed": false' crates/aphrodite/Source/Hook/ToolResult/Inner/Fn.rs` must
drop to 0, and the same six tool results (empty, essential, self, below threshold, live container,
compressed) must produce the same host-visible JSON - read the six reason strings in the new enum
against the six `json!` literals.

### 3.6 A `(Result, bool)` becomes `Result<_, ForwardFailure>`

**When** - a function returns a result paired with a boolean that qualifies it, so an illegal
combination such as "succeeded but timed out" is representable.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs:24`

**Before** -

```rust
) -> (anyhow::Result<reqwest::Response>, bool) {
	let mut UpstreamResult:anyhow::Result<reqwest::Response> = Err(anyhow::anyhow!("unreachable"));
	// ...
	let mut FinalErrorWasTimeout = false;
	// ... terminal arm: `FinalErrorWasTimeout = e.is_timeout(); UpstreamResult = Err(...)`
	(UpstreamResult, FinalErrorWasTimeout)
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Failure/Enum.rs
pub(crate) enum Enum {
	Timeout(anyhow::Error),
	Transport(anyhow::Error),
}
```

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs
) -> Result<reqwest::Response, Forward::Failure::Enum>
```

**Preserved** - the `context("send the upstream request to {url}")` text and the `is_timeout()`
classification are unchanged, and the `UpstreamTimeouts` counter still increments only for the
timeout variant. The fabricated `Err(anyhow::anyhow!("unreachable"))` seed disappears because the
terminal arms return instead of overwriting a placeholder.

**Probe** - `grep -c '"unreachable"' crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs` must
be 0. Then force a connection refusal and a post-send timeout against a dead and a stalling upstream:
`/metrics` must show `UpstreamTimeouts` incremented for the second only.

### 3.7 A retry loop becomes a `RetryDecision`

**When** - the loop body decides between three outcomes (retry, fail, succeed) inside a nested match
on a compound condition.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs:95`

**Before** -

```rust
match Attempt < 3 && e.is_connect() {
	true => {
		let BaseMs = 100 * 2u64.pow(Attempt - 1);
		let Jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75x to 1.25x
		let Ms = (BaseMs as f64 * Jitter) as u64;
		tracing::warn!(Attempt, BackoffMs = Ms, "upstream retry after connect error: {}", e);
		tokio::time::sleep(std::time::Duration::from_millis(Ms)).await;
	},
	false => {
		FinalErrorWasTimeout = e.is_timeout();
		UpstreamResult = Err(anyhow::Error::new(e).context(format!("send the upstream request to {url}")));
		break;
	},
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Decide/Enum.rs
pub(crate) enum Enum {
	Retry { BackoffMs:u64 },
	Fail { Failure:ForwardFailure },
}
```

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Decide/Fn.rs
pub(crate) fn Fn(Attempt:u32, Error:&reqwest::Error) -> Decide::Enum {
	if Attempt < 3 && Error.is_connect() {
		let BaseMs = 100 * 2u64.pow(Attempt - 1);
		let Jitter = rand::random::<f64>() * 0.5 + 0.75;

		return Decide::Enum::Retry { BackoffMs:(BaseMs as f64 * Jitter) as u64 };
	}

	Decide::Enum::Fail { Failure:ForwardFailure::Fn(Error) }
}
```

**Preserved** - the retry condition, the backoff formula, the jitter range and the warn line's fields
are identical, and the policy is still "connect-phase only, at most two retries". The decision is
pure, so the sleep stays in the caller - the only effect the enum carries is the number of
milliseconds to wait.

**Probe** - `grep -c 'is_connect' crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Decide/Fn.rs`
must be 1, and `grep -c 'sleep' crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Decide/Fn.rs`
must be 0 - a sleep inside the decision function would prove the split did not happen.

### 3.8 A terminal decision tree becomes a `TerminalPlan`

**When** - a hook decides between four mutually exclusive outcomes by four successive early returns
from one boolean expression each.

**Site** - `crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs:26`

**Before** -

```rust
match !(State.ChainSplitEnabled && !DoAutoExpand && Content.contains(SegMarker)) {
	true => {
		return None;
	},
	false => {},
}
let Parts = SplitMarkedOutput(Content);
match Parts.len() < 2 {
	true => {
		return None;
	},
	false => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Terminal/Plan/Enum.rs
pub(crate) enum Enum {
	Skip,
	Split { Parts:Vec<(usize, String)> },
}
```

```rust
// crates/aphrodite/Source/Hook/Terminal/Plan/Fn.rs
pub(crate) fn Fn(State:&AphroditeState, Content:&str, DoAutoExpand:bool) -> Plan::Enum {
	if !State.ChainSplitEnabled || DoAutoExpand || !Content.contains(SegMarker) {
		return Plan::Enum::Skip;
	}

	match SplitMarkedOutput(Content) {
		Parts if Parts.len() < 2 => Plan::Enum::Skip,
		Parts => Plan::Enum::Split { Parts },
	}
}
```

**Preserved** - the four skip conditions are evaluated in the same order and short-circuit at the
same point, so a disabled feature still costs no split. Nothing is stored, recorded or previewed
until the plan is `Split`, which is the property that makes the plan safe to compute.

**Probe** - `grep -c 'return None' crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs` must drop
by the two it replaces. Then run the four inputs (feature off, auto-expand on, no segment marker, one
part) and confirm each still produces no marker: `aphrodite_catalog` is unchanged after each.

### 3.9 A poll update becomes `PollEvent` plus `TaskTransition` - BEHAVIOUR

**When** - one poll result is parsed into a single exit code and then applied to every running task,
so the code belongs to a task the poll never named.

**Site** - `crates/aphrodite/Source/PollWorker/Update/Fn.rs:97`

**Before** -

```rust
let Completions:Vec<(usize, i32, String)> = State
	.BgTasks
	.iter()
	.enumerate()
	.filter(|(_, task)| task.Status == BgStatus::Running)
	.filter_map(|(i, task)| EffectiveExit.map(|Ec| (i, Ec, task.AccumulatedOutput.clone())))
	.collect(); // (idx, exit_code, accumulated_output)
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Update/Event/Struct.rs
pub(crate) struct Struct {
	pub TaskID:Option<String>,
	pub ExitCode:Option<i32>,
	pub Output:String,
}
```

```rust
// crates/aphrodite/Source/PollWorker/Update/Transition/Enum.rs
pub(crate) enum Enum {
	Running { Accumulated:BoundedOutput },
	Completed { ExitCode:i32, Accumulated:BoundedOutput },
}
```

**BEHAVIOUR** - today one poll carrying an exit code finishes _every_ running task with that code:
`MaximumBgTasks` is 4, so a second build still in flight is marked done, its accumulator is cleared
and its marker records the other task's exit code (audit D6). After the rewrite the exit code is
attributed to the task the poll result names, and a task the poll did not name keeps running. That is
a correctness fix, so it ships in its own change with this paragraph as its justification.

**Preserved** - the accumulate-then-complete phase split stays: the first pass mutates under the
`BgTasks` borrow, the second needs `&mut AphroditeState` for the marker. `LastPollTurn`, `PollCount`,
`LiveBytesArrived` and `LiveEtaS` are written exactly as before.

**Probe** - `grep -c 'filter_map(|(i, task)| EffectiveExit' crates/aphrodite/Source/PollWorker/Update/Fn.rs`
must be 0. Then background two tasks, poll one to completion, and confirm `aphrodite_stats` reports
one done and one running - the pre-change tree reports two done.

### 3.10 Expiry becomes a `RetentionDecision` with a report

**When** - an expiry pass mutates task status in place and drops entries with `retain`, so nothing
outside can say what expired or why.

**Site** - `crates/aphrodite/Source/PollWorker/Expire/Fn.rs:17`

**Before** -

```rust
pub fn ExpireStaleTasks(State:&mut AphroditeState) {
	let Turn = State.TurnCounter;
	State.BgTasks.iter_mut().for_each(|task| {
		match task.Status == BgStatus::Running && Turn.saturating_sub(task.LastPollTurn) > StaleTurnAge {
			true => {
				task.Status = BgStatus::Stale;
			},
			false => {},
		}
	});

	State.BgTasks.retain(|task| {
		match task.Status {
			BgStatus::Running => true, // keep running tasks
			BgStatus::Done { .. } | BgStatus::Failed { .. } | BgStatus::Stale => Turn.saturating_sub(task.StartedTurn) <= 16,
		}
	});
}
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Expire/Decision/Enum.rs
pub(crate) enum Enum {
	Keep,
	MarkStale,
	Drop,
}
```

```rust
// crates/aphrodite/Source/PollWorker/Expire/Report/Struct.rs
pub(crate) struct Struct {
	pub MarkedStale:Vec<String>,
	pub Dropped:Vec<String>,
}
```

**Preserved** - the two clocks stay separate: a running task is stale after `StaleTurnAge` unpolled
turns, a finished one is dropped 16 turns after it started. `saturating_sub` is unchanged, so a turn
counter that went backwards still yields zero rather than wrapping.

**Probe** - the decision function is pure, so assert it directly for the four boundary cases (turn
exactly at `StaleTurnAge`, one past, exactly 16, one past) and compare against the two in-place
comparisons. `grep -c 'retain' crates/aphrodite/Source/PollWorker/Expire/Fn.rs` must be 1, applying
the report rather than deciding it.

### 3.11 A nudge loop becomes a `NudgePlan`

**When** - a bounded emission pass walks a vector backwards, formats each nudge inline and stops by
returning `Err(())` from a `try_for_each`.

**Site** - `crates/aphrodite/Source/PollWorker/Check/Fn.rs:21`

**Before** -

```rust
let TaskCount = State.BgTasks.len();
let _ = (0..TaskCount).rev().try_for_each(|i| -> Result<(), ()> {
	match Nudged >= 2 {
		true => {
			return Err(());
		},
		false => {},
	}
	let task = &State.BgTasks[i];
	match &task.Status {
		BgStatus::Running => {
			// format and push the nudge
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Check/Plan/Fn.rs
pub(crate) fn Fn(Tasks:&[BgTask]) -> Vec<String> {
	Tasks
		.iter()
		.rev()
		.filter(|Task| Task.Status == BgStatus::Running)
		.take(2)
		.map(Render::Fn::Fn)
		.collect()
}
```

**Preserved** - newest-first, at most two, and stale tasks stay silent. The two-nudge cap is the
per-turn directive budget, so `take(2)` must stay `take(2)` and must stay after the running filter -
taking two before filtering would emit one nudge for a list of stale tasks.

**Probe** - `grep -c 'Nudged >= 2' crates/aphrodite/Source/PollWorker/Check/Fn.rs` must be 0. Then a
state with one running and three stale tasks must produce exactly one nudge, and a state with three
running tasks exactly two, newest first.

### 3.12 Backgrounding becomes a `BackgroundPlan`

**When** - the decision to rewrite a tool call's arguments is made by matching a tuple of a feature
flag and a tool name, then nested further on an argument's value.

**Site** - `crates/aphrodite-hermes/Source/Hook/Call/Dispatch/PreTool/Fn.rs:44`

**Before** -

```rust
let background = match (state.PollWorkerEnabled, call_tool) {
	(true, "terminal" | "process") => {
		let command = input.parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str());
		// For `process(action='poll')`, don't background - it's a check call.
		let is_poll = call_tool == "process" && /* args.action == "poll" */;
		match is_poll {
			true => None,
			false => { /* ShouldBackgroundPre(command) -> the modify payload */ },
		}
	},
	_ => None,
};
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Hook/Call/Dispatch/PreTool/Background/Enum.rs
pub(crate) enum Enum {
	AsIs,
	Background { Args:serde_json::Value },
}
```

```rust
// crates/aphrodite-hermes/Source/Hook/Call/Dispatch/PreTool/Background/Fn.rs
pub(crate) fn Fn(Enabled:bool, Tool:&str, Args:Option<&Value>) -> Background::Enum
```

**Preserved** - the feature flag still gates first, `process(action='poll')` still never backgrounds,
and the emitted `modify` payload still carries `background:true` and `notify_on_complete:true`. The
command string is still the only input to the heuristic, so a tool with no command still returns
`AsIs`.

**Probe** - `grep -c 'is_poll' crates/aphrodite-hermes/Source/Hook/Call/Dispatch/PreTool/Fn.rs` must
be 0, and three calls (`terminal` with a long command, `process` with `action:'poll'`, `read_file`)
must return the same three hook payloads as before - read them against a captured `pre_tool_call`
response.

### 3.13 A tool result becomes a `ToolObservation`

**When** - a pipeline takes four loosely related parameters (raw content, tool name, an optional
classifier hint, call metadata) and threads them through every gate.

**Site** - `crates/aphrodite/Source/Hook/ToolResult/Inner/Fn.rs:25`

**Before** -

```rust
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
```

**After** -

```rust
// crates/aphrodite/Source/Hook/ToolResult/Observation/Struct.rs
pub(crate) struct Struct<'a> {
	pub Content:&'a str,
	pub ToolName:&'a str,
	pub Classify:Option<(&'a str, &'a str)>,
	pub Meta:&'a ToolCallMeta,
}
```

```rust
// crates/aphrodite/Source/Hook/ToolResult/Inner/Fn.rs
pub(crate) fn Fn(State:&mut AphroditeState, Observation:Observation::Struct<'_>) -> Outcome::Enum
```

**Preserved** - every field is borrowed, so the observation costs nothing and nothing outlives the
call. The gates still read the same values in the same order, and `Content` is still the raw string -
not a normalised form - because the threshold gate compares its length.

**Probe** - `grep -c 'Content:&str, ToolName:&str, Classify:Option' crates/aphrodite/Source/Hook/ToolResult`
must be 0 and the struct must hold only borrows (`grep -c "&'a" crates/aphrodite/Source/Hook/ToolResult/Observation/Struct.rs`
equals 3).

### 3.14 Path extraction carries its provenance

**When** - a heuristic extractor returns a bare `Option<String>`, so a consumer cannot tell which
tool matched, which line it came from, or why it did not match.

**Site** - `crates/aphrodite/Source/Hook/Path/Fn.rs:10`

**Before** -

```rust
pub(crate) fn Fn(Content:&str, Tool:&str) -> Option<String> {
	match Tool {
		"read_file" | "write_file" | "patch" => {
			Content.lines().next().and_then(|Line| {
				let Line = Line.trim();
				match Line.starts_with('/') || Line.starts_with("./") {
					true => Some(Line.to_string()),
					false => None,
				}
			})
		},
		// ... `search_files` splits on ':' instead
		_ => None,
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Path/Found/Struct.rs
pub(crate) struct Struct {
	pub Path:String,
	pub Tool:&'static str,
	pub Basis:Basis,
}

// crates/aphrodite/Source/Hook/Path/Basis/Enum.rs
pub(crate) enum Enum {
	FirstLine,
	FirstFieldBeforeColon,
}
```

**Preserved** - the same two tool families are recognised, the same first line is read, and the same
two prefixes are accepted, so the catalog's `ReferencedFiles` set is identical. `Basis` records which
split produced the path, which is what a later bug report needs and what a bare `String` threw away.

**Probe** - `grep -c 'Option<String>' crates/aphrodite/Source/Hook/Path/Fn.rs` must be 0. Then feed
one `read_file` result and one `search_files` result and confirm the catalog still lists exactly the
two paths it listed before, now with the matching `Basis` variant.

### 3.15 A configuration setter parses before the lock

**When** - an ABI setter parses its key and value inside the closure that holds the session lock, so
a parse failure is decided under the lock and an unknown key is indistinguishable from an applied one.

**Site** - `crates/aphrodite/Source/ABI/ConfigurationSet.rs:29`

**Before** -

```rust
match WithState(Hid, |s| {
	match k.as_ref() {
		"model" => s.Model = v.to_string(),
		"engine_threshold_pct" => {
			match v.parse() {
				Ok(n) => s.EngineThresholdPct = n,
				Err(_) => (),
			}
		},
		// ... two more keys, then `_ => {}`
	}
	serde_json::json!({"status":"ok"})
}) {
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Configuration/Update/Enum.rs
pub(crate) enum Enum {
	Model(String),
	EngineThresholdPct(f64),
	ToolThreshold(f64),
	ContextEngineEnabled(bool),
	Unknown,
}
```

```rust
// crates/aphrodite/Source/ABI/ConfigurationSet.rs
let Update = Configuration::Update::Fn::Fn(&k, &v);
match WithState(Hid, |State| Update::Apply::Fn::Fn(State, &Update)) {
```

**Preserved** - the leniency is the contract and must not be tightened: an unknown key still returns
`{"status":"ok"}` and an unparseable number still leaves the previous value. The parse now happens
before the lock is taken, so the lock is held for the assignment only. `context_engine_enabled`
still accepts `true` and `1`.

**Probe** - `grep -c 'v.parse()' crates/aphrodite/Source/ABI/ConfigurationSet.rs` must be 0 (the
parse moved into `Update::Fn::Fn`). Then set `engine_threshold_pct` to `abc` and to `42`: both must
answer `{"status":"ok"}`, and `AphroditeConfigurationGet` must show the old value and then 42.

### 3.16 A reload becomes a `ReloadAssessment` plus a `DiagnosticUpdate`

**When** - a reload computes a value and a diagnostic and then applies both through separate matches
with empty arms, so the assessment and the write are the same block.

**Site** - `crates/aphrodite/Source/ABI/Reload.rs:68`

**Before** -

```rust
match WithState(Hid, |s| {
	match ContextEngineEnabled {
		Some(v) => s.ContextEngineEnabled = v,
		None => (),
	}
	match &ParseError {
		Some(e) => s.ConfigurationError = Some(e.clone()),
		None => (),
	}
	serde_json::json!({"status":"ok", "config_error":ParseError})
}) {
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/CCR/Reload/Assessment/Struct.rs
pub(crate) struct Struct {
	pub ContextEngineEnabled:Option<bool>,
	pub Diagnostic:DiagnosticUpdate,
}

// crates/aphrodite/Source/Proxy/CCR/Reload/Diagnostic/Enum.rs
pub(crate) enum Enum {
	Clear,
	Set(String),
}
```

**Preserved** - a found-but-broken TOML still keeps the previous value, still records the failure on
`ConfigurationError` and still answers `config_error` in the response; a missing file is still a
silent miss. The assessment is computed before `WithState`, so the parse stays outside the lock.

**Probe** - three cases (valid TOML, broken TOML, absent file) must produce the same three
`config_error` payloads as before. `grep -c 'match ContextEngineEnabled' crates/aphrodite/Source/ABI/Reload.rs`
must be 0.

### 3.17 Tool dispatch prepares a command before it dispatches

**When** - a dispatcher parses arguments, looks up a handler and formats errors in one body, so the
parse and the lookup cannot be tested apart and a malformed payload has no name.

**Site** - `crates/aphrodite-hermes/Source/Tool/Dispatch/Fn.rs:33`

**Before** -

```rust
pub fn Fn(name:&str, args_json:&str) -> Value {
	crate::Debug::Init::Fn::Fn();
	let Session = crate::Debug::Session::Last::Fn::Fn();
	let Span = tracing::info_span!("tool_dispatch", tool = name, session = %Session);
	let _Entered = Span.enter();

	let registry = Registry::Fn::Fn();
	match registry.get(name) {
		Some(handler) => {
			let args:Value = match serde_json::from_str(args_json) {
				Ok(v) => v,
				Err(e) => {
					// The message names the failure, never the payload.
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Tool/Dispatch/Command/Struct.rs
pub(crate) struct Struct {
	pub Name:&'static str,
	pub Handler:ToolHandler,
	pub Args:Value,
}
```

```rust
// crates/aphrodite-hermes/Source/Tool/Dispatch/Prepared/Enum.rs
pub(crate) enum Enum {
	Ready(Command::Struct),
	Unknown { Name:String },
	Malformed { Name:String, Error:String },
}
```

**Preserved** - the three host-visible outcomes keep their exact shapes: an unknown tool still
answers `{"error": ...}`, a malformed payload still answers an `error` object whose message names the
failure and never the payload, and the `tool_dispatch` span still opens with the same two fields and
still covers the handler call.

**Probe** - `grep -c 'registry.get(name)' crates/aphrodite-hermes/Source/Tool/Dispatch/Fn.rs` must be
1, inside the preparation step. Then call a tool with `{`, with an unknown name and with valid args:
three responses, read against the pre-change ones.

### 3.18 Registration returns an outcome instead of erasing a warning into `Ok`

**When** - a setup step warns about a non-zero exit and then returns `Ok(())`, so the caller cannot
tell a registered plugin from a refused one.

**Site** - `crates/aphrodite/Source/Setup/Write/Register.rs:24`

**Before** -

```rust
pub(crate) fn Fn(_Context:&SetupContext) -> anyhow::Result<()> {
	let Status = Command::new("hermes")
		.args(["plugins", "enable", "aphrodite"])
		.output()
		.map_err(|e| anyhow::Error::new(e).context("run `hermes plugins enable aphrodite`"))?;

	match !Status.status.success() {
		true => {
			let Stderr = String::from_utf8_lossy(&Status.stderr);
			crate::Logging::Stderr::Fn::Fn(&format!("warning: hermes plugins enable aphrodite: {Stderr}"));
		},
		false => {
			println!("plugin registered with hermes");
		},
	}
	Ok(())
}
```

**After** -

```rust
// crates/aphrodite/Source/Setup/Write/Outcome/Enum.rs
pub(crate) enum Enum {
	Enabled,
	Refused { Stderr:String },
}
```

```rust
// crates/aphrodite/Source/Setup/Write/Register.rs
pub(crate) fn Fn(_Context:&SetupContext) -> anyhow::Result<Write::Outcome::Enum>
```

**Preserved** - the asymmetry is deliberate and stays: a spawn failure is still an `Err` whose
context names the command, and a non-zero exit is still not fatal, because a CLI that ran and refused
has already told the user why on stderr and the plugin may still be discovered on Hermes' next scan.
What changes is that the caller can now report `Refused` instead of claiming success.

**Probe** - `grep -c 'warning: hermes plugins enable' crates/aphrodite/Source/Setup/Write/Register.rs`
must be 0 (the stderr is carried, not printed), and the two paths must still differ: run with a
`hermes` on `PATH` that exits 1, and confirm setup completes with a `Refused` outcome rather than
failing.

### 3.19 A supplied key versus a digest becomes a `ContentLocation`

**When** - a store decides its address with `unwrap_or_else(compute_key)`, so a caller-supplied key
and a content digest are the same `String` and nothing downstream can tell them apart.

**Site** - `crates/aphrodite/Source/Proxy/CCR/Create/Handle/Fn.rs:43`

**Before** -

```rust
let OriginalSize = Request.Content.len();
let hash = Request.Key.unwrap_or_else(|| compute_key(Request.Content.as_bytes()));
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/CCR/Create/Location/Enum.rs
pub(crate) enum Enum {
	Supplied(String),
	Digest(String),
}
```

```rust
// crates/aphrodite/Source/Proxy/CCR/Create/Handle/Fn.rs
let Location = Create::Location::Fn::Fn(Request.Key, Request.Content.as_bytes());
let hash = Location.Hash();
```

**Preserved** - the wire contract is untouched: the response is still the bare hash, and the stored
bytes are still keyed by that hash, so a later `/retrieve` of a caller-supplied key still resolves.
`TokensSaved` still subtracts `hash.len()`, not a marker length.

**Probe** - `grep -c 'unwrap_or_else(|| compute_key' crates/aphrodite/Source/Proxy/CCR/Create/Handle/Fn.rs`
must be 0. Then create one entry with a key and one without, and confirm both hashes resolve through
`/retrieve` and that the response body is still a bare string in both cases.

### 3.20 `&mut Vec<String>` becomes a typed `MetadataField`

**When** - a family extractor appends formatted `key=value` strings to a caller-owned vector, so the
field names and their escaping live in thirteen places.

**Site** - `crates/aphrodite/Source/Proxy/Metadata/Build/Fn.rs:3`

**Before** -

```rust
pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	match Content.contains("error") || Content.contains("aborting") {
		true => {
			Parts.push("status=FAIL".to_string());
		},
		false => {
			Parts.push("status=OK".to_string());
		},
	}
	// ... `Parts.push(format!("files={}", Files))`, `Parts.push(format!("first_err={}", e))`
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Metadata/Field/Enum.rs
pub(crate) enum Enum {
	Status(Status),
	Files(usize),
	FirstError(String),
}
```

```rust
// crates/aphrodite/Source/Proxy/Metadata/Build/Fn.rs
pub(crate) fn Fn(Content:&str) -> Vec<Field::Enum>
```

**Preserved** - the rendered block is byte-identical: `;`-separated `key=value` pairs, `|` escaped to
`/`, the 400-character cut and the trailing-separator trim all stay in the one renderer. The
per-field 80-character `first_err` cut stays where it is, because it is the field's contract, not the
renderer's.

**Probe** - `grep -c '&mut Vec<String>' crates/aphrodite/Source/Proxy/Metadata` must be 0. Then
compare the metadata block for one cargo build log before and after - same string, same order, same
length.

### 3.21 A regex-string protocol becomes a `SignatureMatcher`

**When** - a list of `(kind, pattern-string)` pairs is matched line by line, so a compile error in a
pattern is a runtime non-match and an unused capture allocation is paid per line per pattern.

**Site** - `crates/aphrodite/Source/Stage2/Reduce/Code/Fn.rs:21`

**Before** -

```rust
let Patterns = [
	SigPattern { Kind:"fn", Pattern:r"^(pub\s+)?(async\s+)?fn\s+(\w+)" },
	SigPattern { Kind:"struct", Pattern:r"^(pub\s+)?struct\s+(\w+)" },
	// ... seven more rows
];

Content.lines().for_each(|Line| {
	let Trimmed = Line.trim();
	let Lower = Trimmed.to_lowercase();
	Patterns.iter().for_each(|Sp| {
		match crate::Stage2::Regex::Fn::RegexMatch(Trimmed, &Lower, Sp.Pattern) {
			Some(_Caps) => { /* push "[kind] signature" */ },
			None => {},
		}
	});
});
```

**After** -

```rust
// crates/aphrodite/Source/Stage2/Reduce/Code/Matcher/Enum.rs
pub(crate) enum Enum {
	Fn,
	Struct,
	Enum,
	Trait,
	Impl,
	Def,
	Class,
	Func,
	Function,
}
```

```rust
// crates/aphrodite/Source/Stage2/Reduce/Code/Matcher/Fn.rs
pub(crate) fn Fn(Line:&str, Lower:&str) -> Option<Matcher::Enum>
```

**Preserved** - the same nine declaration shapes are recognised with the same prefix rules, the
`Lower` form is still computed once per line, and the emitted `[kind] signature` strings keep their
spellings. The capture group was never read (`Some(_Caps)`), so dropping it removes an allocation
and no result.

**Probe** - `grep -c 'Pattern:r"' crates/aphrodite/Source/Stage2/Reduce/Code/Fn.rs` must be 0. Then
reduce a file with one of each of the nine declarations and diff the output against the pre-change
reduction - identical lines, identical order.

### 3.22 Declaration recognition becomes a `Recognition` enum

**When** - a scanner answers "is this a declaration?" with a boolean, so "not a declaration" and
"a declaration this scanner cannot parse" are the same answer.

**Site** - `crates/aphrodite/Source/StructExtract/Rust.rs:42`

**Before** -

```rust
let IsFn = (Lower.starts_with("fn ")
	|| Lower.starts_with("pub fn ")
	|| Lower.starts_with("async fn ")
	|| Lower.starts_with("pub async fn ")
	|| Lower.starts_with("pub(crate) fn ")
	|| Lower.starts_with("pub(crate) async fn "))
	&& Trimmed.contains('(');

match IsFn {
	true => {
		let AfterFn = Trimmed.trim_start_matches("pub(crate) ") /* ... */;
		match AfterFn.find('(') {
			Some(Paren) => { /* name, params, return type */ },
			None => {},
		}
	},
	false => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/StructExtract/Rust/Recognition/Enum.rs
pub(crate) enum Enum {
	NotDeclaration,
	Declaration,
	Malformed { Reason:MalformedReason },
}
```

```rust
// crates/aphrodite/Source/StructExtract/Rust/Recognition/Fn.rs
pub(crate) fn Fn(Trimmed:&str, Lower:&str) -> Recognition::Enum
```

**Preserved** - the six accepted prefixes are the same six and `contains('(')` is still required, so
the set of extracted signatures is unchanged. `Malformed` is only reachable for a line that carries a
`fn` prefix and no `(` - which today falls into the `None => {}` arm and vanishes silently; it still
produces no signature, it just says so.

**Probe** - `grep -c 'IsFn' crates/aphrodite/Source/StructExtract/Rust.rs` must be 0. Then extract
from a file containing `pub(crate) async fn Fn(` and one containing `fn ` with no parens: the first
must still yield a signature and the second still none, with the second now reported as `Malformed`.

## Group 4 - types that own an invariant

### 4.1 Storage evidence before marker rendering

**When** - a payload is stored and a marker is rendered for it in the same body, so a caller can
render a marker for content that was never stored.

**Site** - `crates/aphrodite/Source/Hook/Live/Container.rs:38`

**Before** -

```rust
State.InlineStorePut(hash.clone(), Content.to_string());
Chunks.iter().enumerate().for_each(|(i, Chunk)| {
	State.InlineStorePut(LiveChunkKey(&hash, i), (*Chunk).to_string());
});
// ... metadata envelope
let Preview = crate::Preview::Builder::Entry::Fn(TypeStr, Content);
let Marker = Marker::Fn::Fn(&hash, Total, n, &Preview);
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Live/Stored/Struct.rs - private fields, one constructor
pub(crate) struct Struct {
	Hash:String,
	Total:usize,
	Chunks:usize,
}

// crates/aphrodite/Source/Hook/Live/Stored/Fn.rs
pub(crate) fn Fn(State:&mut AphroditeState, Content:&str, TypeStr:&str) -> Option<Stored::Struct>
```

**Preserved** - both stores still happen (the plain hash for the lossless round trip and the chunk
keys for progressive retrieve), in the same order, and the marker is still rendered from the same
`Total` and chunk count. The difference is that the marker can only be rendered from a value whose
existence proves the store ran - the fields are private, so no caller can construct one.

**Probe** - `grep -c 'pub ' crates/aphrodite/Source/Hook/Live/Stored/Struct.rs` must be 0 apart from
the struct itself, and a retrieve of a live marker must still resolve to the exact original bytes -
compare `aphrodite_retrieve` output against the input for a 3-chunk payload.

### 4.2 The poll output cap becomes `BoundedOutput` - BEHAVIOUR

**When** - a length is compared against a cap before an append whose size the comparison never
accounted for, so the cap is a promise the code cannot keep.

**Site** - `crates/aphrodite/Source/PollWorker/Update/Fn.rs:63`

**Before** -

```rust
match task.AccumulatedOutput.len() < 256 * 1024 {
	true => {
		match !task.AccumulatedOutput.is_empty() {
			true => {
				task.AccumulatedOutput.push('\n');
			},
			false => {},
		}
		task.AccumulatedOutput.push_str(Output);
	},
	false => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Update/Output/Struct.rs
pub(crate) struct Struct {
	Buffer:String,
	Maximum:usize,
}

impl Struct {
	pub(crate) fn Push(&mut self, Content:&str) {
		let Room = self.Maximum.saturating_sub(self.Buffer.len());

		self.Buffer.extend(Content.chars().take(Room));
	}
}
```

**BEHAVIOUR** - today the test precedes a `push_str` of the whole poll payload, so one poll can append
its entire output and the accumulator exceeds the stated 256 KB by up to one payload, after which
`LiveBytesArrived` reports the over-cap length as bytes arrived (audit D4). After the rewrite the
append is cut to the remaining room, so the cap holds at every point and the reported byte count is
never above it. A reader of `aphrodite_stats` sees a smaller `LiveBytesArrived` once the cap binds;
that is the fix, not a regression.

**Preserved** - the newline separator is still inserted only when the buffer is non-empty, the cap is
still 256 KB, and the first 256 KB of any accumulation is byte-identical to what the old code kept.

**Probe** - `grep -c '256 \* 1024' crates/aphrodite/Source/PollWorker/Update/Fn.rs` must be 1, in the
constructor. Then poll a task with a 400 KB single result: `LiveBytesArrived` must read 262144, not 409600.

### 4.3 A reducer sample becomes `CountedSample<T, N>`

**When** - a reducer collects every matching line into three vectors, keeps the total only to render
a count, and then throws most of the sample away with `take`.

**Site** - `crates/aphrodite/Source/Stage2/Reduce/Build/Fn.rs:16`

**Before** -

```rust
let mut Errors:Vec<String> = Vec::new();
let mut Warnings:Vec<String> = Vec::new();
let mut SummaryLines:Vec<String> = Vec::new();
// ... push on match, then:
match !Errors.is_empty() {
	true => {
		Parts.push(format!("[{} errors]", Errors.len()));
		Parts.extend(Errors.into_iter().take(20));
	},
	false => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/Stage2/Reduce/Sample/Struct.rs
pub(crate) struct Struct<T, const N:usize> {
	Sample:Vec<T>,
	Total:usize,
}

impl<T, const N:usize> Struct<T, N> {
	pub(crate) fn Push(&mut self, Item:T) {
		self.Total += 1;

		if self.Sample.len() < N {
			self.Sample.push(Item);
		}
	}
}
```

**Preserved** - the headline counts stay exact because `Total` counts every match, not the sample,
and the sample still holds the first N in input order - which is what the renderer shows. The three
caps (10 summaries, 20 errors, 10 warnings) become the const parameter, so they are visible at the
declaration instead of at the render.

**Probe** - `grep -c 'take(20)' crates/aphrodite/Source/Stage2/Reduce/Build/Fn.rs` must be 0. Then
reduce a log with 100 errors: the header must still read `[100 errors]` and the body must still carry
the first 20.

### 4.4 An extraction budget becomes `ExtractionBudget` plus `BudgetPolicy`

**When** - a scan shares one `&mut isize` between several categories, and the sign test that stops it
is the only place the policy exists.

**Site** - `crates/aphrodite/Source/StructExtract/Rust.rs:26`

**Before** -

```rust
pub(super) fn Fn(Content:&str, Result:&mut HashMap<String, Vec<String>>, Budget:&mut isize) {
	// ...
	Fns.push(s);
	*Budget -= Slen as isize + 1;
	// ... later, between categories:
	match *Budget <= 0 {
```

**After** -

```rust
// crates/aphrodite/Source/StructExtract/Budget/Struct.rs
pub(crate) struct Struct {
	Remaining:isize,
	Policy:Policy,
}

// crates/aphrodite/Source/StructExtract/Budget/Policy/Enum.rs
pub(crate) enum Enum {
	SpendAndStop,
	SpendAndContinue,
}
```

**Preserved** - the accounting is identical: a signature costs its length plus one, the budget is
shared across the categories in the documented order, and the scan stops the moment it is spent. The
`+1` is the separator the renderer will insert, so it must stay in the charge and not move to the
render.

**Probe** - `grep -c '&mut isize' crates/aphrodite/Source/StructExtract` must be 0. Then extract from
a file whose signatures total just under and just over the budget: the under case must still list
every category and the over case must still stop at the same signature.

### 4.5 `limit == 0` becomes `PreviewLimit::{Unlimited, Characters(NonZeroUsize)}`

**When** - zero means "no limit" in one field and "nothing" everywhere else in the language, so every
consumer has to remember which reading applies.

**Site** - `crates/aphrodite/Source/Preview/Builder/Cap.rs:16`

**Before** -

```rust
pub(crate) fn Fn(Preview:&str, MaximumChars:usize) -> String {
	match MaximumChars == 0 || Preview.chars().count() <= MaximumChars {
		true => {
			return Preview.to_string();
		},
		false => {},
	}
```

**After** -

```rust
// crates/aphrodite/Source/Preview/Limit/Enum.rs
pub(crate) enum Enum {
	Unlimited,
	Characters(std::num::NonZeroUsize),
}
```

```rust
// crates/aphrodite/Source/Preview/Builder/Cap.rs
pub(crate) fn Fn(Preview:&str, Limit:Limit::Enum) -> Cow<'_, str>
```

**Preserved** - an absent or zero cap still returns the preview unchanged and unbounded, the closing
`]` is still preserved when the preview is self-bracketed, and `MaximumChars <= 3` still collapses to
a plain character cut. The translation from the configured `usize` to the enum happens once, where
the configuration is read, so `0` is converted to `Unlimited` at exactly one place.

**Probe** - `grep -c 'MaximumChars == 0' crates/aphrodite/Source/Preview` must be 0. Then set
`preview_max_chars = 0` and confirm the preview is untruncated, and set it to 3 and confirm the
collapse path still runs (`grep -c '...'` on the rendered preview is 0).

### 4.6 A handle table becomes a `HandleRegistry`

**When** - a global table is handed out as a raw guard, so lazy initialisation, poison recovery and
allocation are the caller's problem at every export.

**Site** - `crates/aphrodite/Source/ABI/Handle/Table.rs:32`

**Before** -

```rust
pub(crate) fn Handles() -> std::sync::MutexGuard<'static, Option<HashMap<usize, Arc<Mutex<AphroditeState>>>>> {
	let mut g = HandleTable.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	match g.is_none() {
		true => *g = Some(HashMap::new()),
		false => (),
	}
	g
}
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Handle/Registry/Struct.rs
pub(crate) struct Struct;

impl Struct {
	pub(crate) fn Insert(&self, State:Arc<Mutex<AphroditeState>>) -> usize;
	pub(crate) fn Lookup(&self, Handle:usize) -> Option<Arc<Mutex<AphroditeState>>>;
	pub(crate) fn Remove(&self, Handle:usize) -> Option<Arc<Mutex<AphroditeState>>>;
}
```

**Preserved** - the table is still a single process-global mutex, still lazily initialised, still
recovers a poisoned lock rather than propagating it, and the per-session `Arc` is still cloned out of
the map so the global lock is never held across a hook body. Handle values still come from the same
allocator, so an existing handle remains valid.

**Probe** - `grep -rn 'Handles()' crates/aphrodite/Source/ABI` must be 0 (every caller now uses a
named operation), and `python3 Maintain/check_ffi_contract.py` must still pass, since the exported
symbols and their arities are untouched.

### 4.7 A task vector becomes a `TaskRegistry`

**When** - a `Vec<BgTask>` in the session state is pushed, iterated, filtered, mutated and retained
from five modules, so capacity and lifecycle have no owner.

**Site** - `crates/aphrodite/Source/PollWorker/Task/Struct.rs:14`

**Before** -

```rust
pub struct BgTask {
	pub ID:String,
	pub Tool:String,
	pub command:String,
	pub StartedTurn:usize,
	// ... and the state holds `BgTasks: Vec<BgTask>`, mutated from
	// `Insert`, `Update`, `Check`, `Expire` and `Status`
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Registry/Struct.rs
pub(crate) struct Struct {
	Tasks:Vec<BgTask>,
	Maximum:usize,
}

impl Struct {
	pub(crate) fn Insert(&mut self, Task:BgTask) -> Option<usize>;
	pub(crate) fn Running(&self) -> impl Iterator<Item = &BgTask>;
	pub(crate) fn Retain(&mut self, Keep:impl Fn(&BgTask) -> bool);
}
```

**Preserved** - insertion order is still the vector order, and the nudge pass still walks it
newest-first, so `Rev()` over the same storage must be what `Running()` feeds. `MaximumBgTasks` is
still 4 and the eviction rule is unchanged - the registry owns it, but it does not re-decide it.

**Probe** - `grep -rn 'State.BgTasks' crates/aphrodite/Source` must be 0 outside the registry, and
`grep -c 'MaximumBgTasks' crates/aphrodite/Source/PollWorker/Registry/Struct.rs` must be 1. Then
insert five tasks with the cap at four and confirm the eviction victim is the same one as before.

### 4.8 A double-serialised body becomes `FinalBody` over `Bytes`

**When** - the same JSON value is serialised twice in one body: once to measure it and once to send
it, so a large response is encoded twice and both copies are live at the same moment.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs:204`

**Before** -

```rust
let CompLength = serde_json::to_vec(&Compressed).map(|v| v.len()).unwrap_or(0);
// ... the dev log uses `CompLength`
let body = serde_json::to_vec(&Compressed).unwrap_or_else(|_| ResponseBody.to_vec());
// ... cache put, then the response body
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Body/Struct.rs
pub(crate) struct Struct(bytes::Bytes);

impl Struct {
	pub(crate) fn Fn(Compressed:&serde_json::Value, Fallback:bytes::Bytes) -> Self;
	pub(crate) fn Length(&self) -> usize;
	pub(crate) fn Bytes(&self) -> bytes::Bytes;
}
```

**Preserved** - the serialisation error still falls back to the original upstream body, the length
reported to the dev log is still the serialised length (not the marker length), and the response body
and the cache entry still carry identical bytes. `bytes::Bytes` clones are refcount bumps, so the
cache write and the response share one allocation.

**Probe** - `grep -c 'serde_json::to_vec(&Compressed)' crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs`
must be 1. Then compare a compressed response's body bytes and its cached bytes - they must be equal,
and the dev log's `CompressedLength` must be unchanged.

## Group 5 - preparation before effects

### 5.1 Prepare the segments, then apply them

**When** - one loop stores each segment, records its marker and builds its JSON entry, so a failure
half way through leaves the store holding segments no caller was told about.

**Site** - `crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs:44`

**Before** -

```rust
Parts.iter().for_each(|(Index, SegText)| {
	TotalOrig += SegText.len();
	let Ct = headroom_core::transforms::content_detector::detect_content_type(SegText).content_type;
	let TypeStr = Ct.as_str().to_string();
	let SegHash = headroom_core::ccr::compute_key(SegText.as_bytes());
	State.InlineStorePut(SegHash.clone(), SegText.to_string());
	SegHashes.push(SegHash.clone());
	let mut Preview = crate::Preview::Builder::Entry::Fn(&TypeStr, SegText);
	// ... error hint, marker render, RecordMarker, Segments.push
});
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Terminal/Segment/Struct.rs - one prepared segment, nothing stored yet
pub(crate) struct Struct {
	pub Index:usize,
	pub Hash:String,
	pub TypeStr:String,
	pub Size:usize,
	pub Preview:String,
	pub Marker:String,
}
```

```rust
// crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs
let Prepared:Vec<Segment::Struct> = Parts.iter().map(Segment::Fn::Fn).collect();
let Payload = Apply::Fn::Fn(State, &Prepared);
```

**Preserved** - the per-segment order is the same (detect, hash, preview, error hint, marker), the
totals `TotalOrig` and `TotalMarker` still sum the same numbers, and the JSON payload still carries
the same six keys per segment. What moves is that the store writes happen only after every segment is
prepared, so a panic or an early return cannot leave half a split in the store.

**Probe** - `grep -c 'InlineStorePut' crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs` must be
1, inside the apply step. Then split a three-segment output: `aphrodite_catalog` must list three
entries and the payload must carry three markers, as before.

### 5.2 `BindOne` extracted from the stream pipeline

**When** - a `then(|(name, cli)| async move { ... })` closure holds path resolution, directory
creation, bind and state construction, so none of it can be read or checked on its own.

**Site** - `crates/aphrodite/Source/Boot/Listen/Bind.rs:34`

**Before** -

```rust
let Bound:Vec<Bound> = futures::stream::iter(Proxies)
	.then(|(name, mut CLI)| {
		async move {
			// resolve a relative ccr_db_path against the executable directory
			// create its parent directory
			let Listener = tokio::net::TcpListener::bind(CLI.Listen).await.with_context(|| {
				format!("failed to bind listener \"{name}\" on {}", CLI.Listen)
			})?;
			let State = State::Build::Build(&CLI, Compression).await?;
			Ok::<Bound, anyhow::Error>(Bound { name, CLI, Listener, State })
		}
	})
	.try_collect()
	.await?;
```

**After** -

```rust
// crates/aphrodite/Source/Boot/Listen/Bind/One/Fn.rs - declared `pub mod One;` in Bind/mod.rs
pub(crate) async fn Fn(
	Name:String,
	mut CLI:CLI,
	Compression:Option<&CompressionConfiguration>,
) -> anyhow::Result<Bound>
```

```rust
// crates/aphrodite/Source/Boot/Listen/Bind/All/Fn.rs
futures::stream::iter(Proxies)
	.then(|(Name, CLI)| Bind::One::Fn::Fn(Name, CLI, Compression))
	.try_collect()
	.await
```

**Preserved** - binding still happens before any listener serves, the failure still aborts startup
entirely, the context string still names the listener and its address, and `try_collect` still stops
at the first error. The stream stays sequential (`then`, not `then_concurrent`), because two
listeners binding in parallel would make the error message depend on scheduling.

**Probe** - `grep -c 'TcpListener::bind' crates/aphrodite/Source/Boot/Listen/Bind.rs` must be 0 and 1
in `Bind/One/Fn.rs`. Then start the binary with one port already taken: it must still exit non-zero
with `failed to bind listener "<name>" on <addr>`.

### 5.3 A guard released before a locking helper is called

**When** - a guard on a static is bound to a named local and a helper that locks the same static is
called while it is alive, so the call blocks on a mutex the same thread already holds.

**Site** - `crates/aphrodite-hermes/Source/Debug/Session/Record/Fn.rs:52`

**Before** -

```rust
let mut map = SESSION_PARENTS
	.get_or_init(|| Mutex::new(HashMap::new()))
	.lock()
	.unwrap_or_else(std::sync::PoisonError::into_inner);
map.entry(session.to_string()).or_insert_with(|| parent.to_string());

let root = Chain::Fn::Fn(session).last().cloned().unwrap_or_else(|| session.to_string());
aphrodite::Debug::Root::Note::Fn::Fn(&root);
```

**After** -

```rust
// crates/aphrodite-hermes/Source/Debug/Chain/In/Fn.rs - pure over a supplied map, locks nothing
pub(crate) fn Fn(Map:&HashMap<String, String>, Session:&str) -> Vec<String>
```

```rust
// crates/aphrodite-hermes/Source/Debug/Session/Record/Fn.rs
let Root = {
	let mut Map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);

	Map.entry(Session.to_owned()).or_insert_with(|| Parent.to_owned());

	Chain::In::Fn::Fn(&Map, Session).last().cloned()
};

aphrodite::Debug::Root::Note::Fn::Fn(&Root.unwrap_or_else(|| Session.to_owned()));
```

**Preserved** - one acquisition, one scope: the edge is inserted and the root resolved from the same
guard, so the walk sees the edge it just inserted. The core logger is told the root after the block
ends, so the guard is out of scope before any other call. The locking wrapper `Chain::Fn::Fn` stays
available for callers that hold no guard.

**Probe** - `grep -n -B3 'Chain::Fn::Fn' crates/aphrodite-hermes/Source/Debug/Session/Record/Fn.rs`
must show no live guard in the enclosing scope. Then record a chain of three sessions and confirm the
published root is the root-most ancestor - `aphrodite_debug` writes `debug.<root>`, so the file name
is the assertion.

The working tree already carries this rewrite; the before block is the shape at the audited revision,
kept here because it is the shape the rule forbids and the reason the two-leaf split exists.

### 5.4 One `ExportJSON` adapter owns panic interception, conversion and transfer

**When** - every C export repeats the same three steps: call the state closure, render `Ok` as JSON,
render `Err` as an error payload, and transfer ownership of the string.

**Site** - `crates/aphrodite/Source/ABI/ConfigurationSet.rs:48`

**Before** -

```rust
	match WithState(Hid, |s| { /* ... */ serde_json::json!({"status":"ok"}) }) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Export/JSON/Fn.rs
pub(crate) fn Fn(Operation:impl FnOnce() -> anyhow::Result<serde_json::Value>) -> *mut c_char {
	// catch_unwind -> error conversion -> serialisation -> ownership transfer
}
```

```rust
// crates/aphrodite/Source/ABI/ConfigurationSet.rs
ABI::Export::JSON::Fn::Fn(|| {
	WithState(Hid, |State| { /* ... */ Ok(serde_json::json!({"status":"ok"})) })
})
```

**Preserved** - the error payload is still `format!("{e:#}")`, a panic still becomes
`"internal error: panicked"` rather than an unwind across `extern "C"`, and the returned pointer is
still a `CString::into_raw` the host frees with `AphroditeFreeString`. The adapter never changes an
export's signature, because the symbol is the ABI.

**Probe** - `grep -rc 'ToJSONError' crates/aphrodite/Source/ABI/*.rs` must show every export calling
the adapter rather than the two renderers directly, and `python3 Maintain/check_ffi_contract.py` must
still pass. Then call one export with a null handle: the payload must still be the same JSON error.

### 5.5 The watcher split into event source, debounce controller and session apply

**When** - one async block creates the watcher, debounces, parses the config, applies it to every
session and renders the log line, so the only way to test the debounce is to touch a file.

**Site** - `crates/aphrodite/Source/Boot/Listen/Watcher.rs:41`

**Before** -

```rust
tokio::spawn(async move {
	let (Tx, mut Rx) = tokio::sync::mpsc::channel(16);
	let mut Watcher = match notify::recommended_watcher(move |Res:Result<Event, notify::Error>| {
		// filter to `aphrodite.toml` Modify events, `Tx.try_send(())`
	}) { /* ... */ };
	// ... watch the directory
	loop {
		let Received = Rx.recv().await.is_some();
		match Received {
			true => { /* debounce, drain, load, apply thresholds, log */ },
			false => {},
		}
	}
});
```

**After** -

```rust
// crates/aphrodite/Source/Boot/Listen/Watcher/Source/Fn.rs - events in, `()` out, no config knowledge
pub(crate) fn Fn(Path:PathBuf) -> Option<tokio::sync::mpsc::Receiver<()>>

// crates/aphrodite/Source/Boot/Listen/Watcher/Debounce/Fn.rs - collapse a burst into one tick
pub(crate) async fn Fn(Rx:&mut tokio::sync::mpsc::Receiver<()>)

// crates/aphrodite/Source/Boot/Listen/Watcher/Apply/Fn.rs - load, resolve, store, log
pub(crate) fn Fn(States:&[Arc<AppState>], Path:&str)
```

**Preserved** - the 500 ms settle, the drain of events accumulated during it, the filter to
`aphrodite.toml` `Modify` events, and the fact that a failed load only warns and keeps the previous
thresholds. The three `store(..., Ordering::Relaxed)` calls and the log line's fields stay as they
are, and the apply still runs once per burst rather than once per event.

**Probe** - `grep -c 'tokio::spawn' crates/aphrodite/Source/Boot/Listen/Watcher.rs` must be 1, in the
composition only. Then touch the config file five times within 200 ms and confirm exactly one
`config reloaded` line - the debounce is now readable without a filesystem.

### 5.6 An owned prepared command replaces a `RefCell` in the ABI dispatch

**When** - a value computed before a closure is shared into it through a `RefCell`, so the closure's
borrow rules decide how the dispatch is written rather than the dispatch deciding.

**Site** - `crates/aphrodite/Source/ABI/Dispatch.rs:71`

**Before** -

```rust
let PrefetchOutcomes = std::cell::RefCell::new(match name == "prefetch" {
	true => {
		let Paths = Args.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
		let PathStrings:Vec<String> = Paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
		Some(crate::Prefetch::Read::Entry::Fn::ReadPaths(&PathStrings))
	},
	false => None,
});

let (Result, Succeeded) = match WithState(Hid, |s| {
	match name.as_ref() {
```

**After** -

```rust
// crates/aphrodite/Source/ABI/Dispatch/Prepared/Struct.rs
pub(crate) struct Struct {
	pub Prefetch:Option<PrefetchOutcome>,
	pub Hook:Option<Hook::Name>,
}
```

```rust
// crates/aphrodite/Source/ABI/Dispatch/Fn.rs
let Prepared = Dispatch::Prepared::Fn::Fn(&name, &Args)?;
let (Result, Succeeded) = match WithState(Hid, |State| Dispatch::Apply::Fn::Fn(State, Prepared, Content, Tool)) {
```

**Preserved** - the load-bearing property is F9 and it does not move: the prefetch files are still
read from disk **before** the `HANDLES` lock is taken, so no other session queues behind a cold-mount
read. The `prefetch` arm still consumes the outcomes exactly once, and every other arm still reaches
its hook with the same `Content` and `Tool`.

**Probe** - `grep -c 'RefCell' crates/aphrodite/Source/ABI/Dispatch.rs` must be 0, and
`grep -n -A2 'ReadPaths' crates/aphrodite/Source/ABI/Dispatch/Prepared/Fn.rs` must show the read
outside any `WithState` call. Then call the `prefetch` host call twice and confirm the second call
re-reads rather than reusing a cached outcome.

## Group 6 - parse, summarise, render

### 6.1 Preview builders become `Parse` then `Summary` then `Render`

**When** - one function filters lines, tallies them and formats the result, so the parse rule and the
output shape cannot be changed independently.

**Site** - `crates/aphrodite/Source/Preview/Builder/Ls.rs:14`

**Before** -

```rust
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	use std::collections::HashMap;
	let mut Files = 0usize;
	let mut dirs = 0usize;
	let mut Ext:HashMap<String, usize> = HashMap::new();

	Inp.raw
		.lines()
		.filter(|Line| { /* skip the `total N` header */ })
		.for_each(|Line| {
			// long-form vs short-form detection, then tally
		});
	// ... render from the tallies
}
```

**After** -

```rust
// crates/aphrodite/Source/Preview/Builder/Ls/Summary/Struct.rs
pub(crate) struct Struct {
	pub Files:usize,
	pub Directories:usize,
	pub Extensions:Vec<(String, usize)>,
}

// crates/aphrodite/Source/Preview/Builder/Ls/Parse/Fn.rs  -> Summary::Struct
// crates/aphrodite/Source/Preview/Builder/Ls/Render/Fn.rs -> String
```

**Preserved** - the same lines are skipped (the `total N` header, blanks), the same long-form mode
string test is applied, the same extensions are counted, and the rendered string is character-for-
character what the builder produced. The extension ordering must stay the same - it comes from the
tally, so the sort has to move with it, not be re-derived at render.

**Probe** - `grep -c 'fn Fn' crates/aphrodite/Source/Preview/Builder/Ls/Fn.rs` must be 0 and the three
leaves must each hold one export. Then preview one `ls -l` output and one `ls` output and compare
against the pre-change strings.

### 6.2 A git log becomes a record-boundary state machine - BEHAVIOUR

**When** - a record's subject is found by scanning forward for the first line that is not a header,
and the scan is allowed to cross into the next record.

**Site** - `crates/aphrodite/Source/Preview/Builder/GitLog.rs:24`

**Before** -

```rust
let Subject = all[i + 1..]
	.iter()
	.map(|l| l.trim())
	.find(|l| {
		!l.is_empty()
			&& !l.starts_with("Author:")
			&& !l.starts_with("Date:")
			&& !l.starts_with("Merge:")
			&& !l.starts_with("commit ")
	})
	.unwrap_or("")
	.chars()
	.take(32)
	.collect::<String>();
```

**After** -

```rust
// crates/aphrodite/Source/Preview/Builder/GitLog/Record/Struct.rs
pub(crate) struct Struct {
	pub Hash:String,
	pub Subject:String,
}

// crates/aphrodite/Source/Preview/Builder/GitLog/Parse/Fn.rs
pub(crate) fn Fn(Raw:&str) -> Vec<Record::Struct>
```

**BEHAVIOUR** - today the subject search rejects a `commit ` line instead of ending the record, so it
continues into the next commit's block: a commit with an empty message inherits the _following_
commit's subject (audit D5). After the rewrite the parse stops at the next `commit ` boundary, so a
subject-less commit yields an empty subject. The first and last endpoint of a log preview therefore
name the right change. A stricter parse, so it ships alone.

**Preserved** - the commit count, the seven-character short hash, the 32-character subject cut, and
the header prefixes that are skipped (`Author:`, `Date:`, `Merge:`) are unchanged. Only the record
boundary is now a boundary.

**Probe** - `grep -c 'starts_with("commit ")' crates/aphrodite/Source/Preview/Builder/GitLog/Parse/Fn.rs`
must be 1, as a terminator rather than a filter. Then preview a two-commit log whose first commit has
an empty message: the first endpoint must be empty, not the second commit's subject.

### 6.3 One `ContentAnalysis` serves both the preview and the metadata renderer

**When** - the same content is re-detected by two renderers, and one of them dispatches on a `&str`
type name with thirteen literal arms.

**Site** - `crates/aphrodite/Source/Proxy/Metadata/Fn.rs:17`

**Before** -

```rust
match Ct {
	"code_rust" => Rust::Fn::Fn(Content, LineCount, &mut Parts),
	"code_python" => Python::Fn::Fn(Content, LineCount, &mut Parts),
	"code_go" => Go::Fn::Fn(Content, LineCount, &mut Parts),
	"code_js" => JS::Fn::Fn(Content, LineCount, &mut Parts),
	"code" => Code::Fn::Fn(Content, LineCount, &mut Parts),
	"error" => Error::Fn::Fn(Content, &mut Parts),
	// ... seven more arms, then `_ => Text::Fn::Fn(...)`
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Analysis/Enum.rs
pub(crate) enum Enum {
	Rust, Python, Go, JS, Code, Error, Diff, Git, BuildOutput, Log, Linter, JSON, Text,
}

// crates/aphrodite/Source/Proxy/Analysis/Fn.rs
pub(crate) fn Fn(Type:&str) -> Analysis::Enum
```

**Preserved** - the thirteen names and their spellings are unchanged, so the `Ct` a caller passes
still selects the same family, and the `_` arm still falls back to `Text`. The detection happens once
and the enum is matched by both renderers, so an unrecognised type can no longer be classified one
way by the preview and another by the metadata.

**Probe** - `grep -c '"code_rust"' crates/aphrodite/Source` must be 2 (the detector and
`Analysis::Fn::Fn`), not 3. Then compress one payload of each family and compare the preview and the
metadata block against the pre-change output.

### 6.4 A UTF-8 chunk iterator replaces a mutable collector

**When** - a function pushes borrowed slices into a `Vec` from inside a `for_each` that carries a
mutable cursor, so the chunking rule and the accumulation are interleaved.

**Site** - `crates/aphrodite/Source/Hook/Live/Chunk/Fn.rs:11`

**Before** -

```rust
pub(crate) fn Fn(Content:&str, ChunkSize:usize) -> Vec<&str> {
	let mut Chunks = Vec::new();
	let mut Start = 0usize;
	Content.char_indices().for_each(|(Index, _Character)| {
		match Index >= Start + ChunkSize {
			true => {
				Chunks.push(&Content[Start..Index]);
				Start = Index;
			},
			false => {},
		}
	});
	match Start < Content.len() {
		true => {
			Chunks.push(&Content[Start..]);
		},
		false => {},
	}
	Chunks
}
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Live/Chunk/Fn.rs
pub(crate) fn Fn(Content:&str, ChunkSize:usize) -> impl Iterator<Item = &str> {
	Content
		.char_indices()
		.map(|(Index, _Character)| Index)
		.chain(std::iter::once(Content.len()))
		.scan((0usize, 0usize), move |(Start, Seen), Index| {
			// emit a boundary once the run reaches `ChunkSize`
		})
}
```

**Preserved** - every chunk is still a slice of `Content` (nothing is copied), the cut still lands on
a character boundary, and the reassembly is still lossless: concatenating the chunks reproduces
`Content` byte for byte. A chunk may still be shorter than `ChunkSize` when a multi-byte character
straddles the boundary - that is the price of never splitting one, not a defect.

**Probe** - for a 10 KB payload of mixed ASCII and multi-byte text, `chunks.concat()` must equal the
input and `chunks.len()` must be unchanged. Then retrieve the live marker and compare against the
original.

### 6.5 `floor_char_boundary` replaces a hand-written boundary walk

**When** - a byte index is walked backwards looking for a character boundary, which is a standard
library operation spelled by hand.

**Site** - `crates/aphrodite/Source/Proxy/Body/Truncate/Fn.rs:12`

**Before** -

```rust
pub(crate) fn Fn(s:&str, Maximum:usize) -> &str {
	match s.len() <= Maximum {
		true => s,
		false => &s[..(0..=Maximum).rev().find(|&b| s.is_char_boundary(b)).unwrap_or(0)],
	}
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Body/Truncate/Fn.rs
pub(crate) fn Fn(Content:&str, Maximum:usize) -> &str {
	if Content.len() <= Maximum {
		return Content;
	}

	&Content[..Content.floor_char_boundary(Maximum)]
}
```

**Preserved** - the cut is the same index: the greatest boundary at or below `Maximum`, and a string
shorter than `Maximum` is still returned whole. `floor_char_boundary` is the standard library's
version of the same search, so the returned slice is identical for every input - which is the whole
reason this recipe is safe.

**Probe** - for each of `Maximum` in `0..=len` over a string with a 3-byte character at the boundary,
the returned slice must equal the old expression's. `floor_char_boundary` requires a recent toolchain;
if the pinned one lacks it, keep the walk and record that in the change.

## Group 7 - ownership and cost

### 7.1 `Cow` for an optional text transform

**When** - a preview is rebuilt with `format!` to append a hint, so the unchanged case pays an
allocation and a copy for a string that was already correct.

**Site** - `crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs:60`

**Before** -

```rust
let mut Preview = crate::Preview::Builder::Entry::Fn(&TypeStr, SegText);
match !Preview.contains("error[") {
	true => {
		match SegmentErrorHint(SegText) {
			Some(Hint) => {
				let Inner = Preview.trim_end_matches(']');
				Preview = format!("{} ⚠ {}]", Inner, Hint);
			},
			None => {},
		}
	},
	false => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/Hook/Terminal/ChainSplit/Hint/Fn.rs
pub(crate) fn Fn(Preview:&str, Segment:&str) -> Cow<'_, str> {
	if Preview.contains("error[") {
		return Cow::Borrowed(Preview);
	}

	match SegmentErrorHint(Segment) {
		Some(Hint) => Cow::Owned(format!("{} ⚠ {}]", Preview.trim_end_matches(']'), Hint)),
		None => Cow::Borrowed(Preview),
	}
}
```

**Preserved** - the hint is still merged inside the brackets, never appended after the `]`, and it is
still skipped for a preview that already carries `error[`. The string that reaches the model is
identical in every case; only the two unchanged cases stop allocating.

**Probe** - `grep -c 'format!("{} ⚠ {}]"' crates/aphrodite/Source/Hook/Terminal/ChainSplit/Fn.rs` must
be 0 (it moved into the `Cow::Owned` arm). Then split a segment with a hint and one without: the
markers must be byte-identical to the pre-change ones.

### 7.2 `std::mem::take` for completion ownership

**When** - a completion path clones an accumulated buffer and then clears the original, so the copy
exists only to move the value.

**Site** - `crates/aphrodite/Source/PollWorker/Update/Fn.rs:121`

**Before** -

```rust
.filter_map(|(i, task)| EffectiveExit.map(|Ec| (i, Ec, task.AccumulatedOutput.clone())))
.collect(); // (idx, exit_code, accumulated_output)
// ... later, inside the completion:
task.AccumulatedOutput.clear();
```

**After** -

```rust
// crates/aphrodite/Source/PollWorker/Update/Fn.rs
let Completions:Vec<(usize, i32, String)> = State
	.BgTasks
	.iter_mut()
	.enumerate()
	.filter(|(_, Task)| Task.Status == BgStatus::Running)
	.filter_map(|(Index, Task)| EffectiveExit.map(|ExitCode| (Index, ExitCode, std::mem::take(&mut Task.AccumulatedOutput))))
	.collect();
```

**Preserved** - the buffer is still emptied, and it is emptied at the same point in the sequence: the
size read before the completion (`Sz`) must come from the moved string, not from the emptied field.
`LiveBytesArrived` still reports the final size. `String::take` leaves an empty string with no
capacity, which is the same state `clear()` left.

**Probe** - `grep -c 'AccumulatedOutput.clone()' crates/aphrodite/Source/PollWorker/Update/Fn.rs` must
be 0. Then complete a 256 KB task: the marker's `Size` must still be the pre-clear length and the
task's accumulator must still read 0.

### 7.3 `Bytes` for a body that is read twice

**When** - the same response body is copied once for the cache and once for the caller, so a large
body is held in three allocations at the peak.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs:297`

**Before** -

```rust
match State.ResponseCache.lock() {
	Ok(mut Cache) => {
		Cache.put(Ck, (std::time::Instant::now(), ResponseBody.to_vec()));
	},
	Err(_) => {},
}
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs
Cache.put(Ck, (std::time::Instant::now(), ResponseBody.clone()));
```

**Preserved** - the cache still stores the raw upstream body, not the compressed one, and only on a
successful response under the size limit. The clone is a refcount bump once the extractor hands back
`bytes::Bytes`, which it already does - `ResponseBody` is a `Bytes` in this handler, so `to_vec()` was
the copy. A cache hit still replays byte-identical content.

**Probe** - `grep -c 'ResponseBody.to_vec()' crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs`
must be 0. Then request the same non-streaming completion twice with the cache on: the second
response's bytes must equal the first's.

### 7.4 Serialise once, then share

**When** - the serialised form is computed in two places because one of them only needs its length.

**Site** - `crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs:190`

**Before** -

```rust
let CompLength = serde_json::to_vec(&Compressed).map(|v| v.len()).unwrap_or(0);
tracing::info!(/* ... */ CompressedLength = CompLength, /* ... */ "<<< COMPRESSED");
// ...
let body = serde_json::to_vec(&Compressed).unwrap_or_else(|_| ResponseBody.to_vec());
```

**After** -

```rust
// crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs
let Final = FinalBody::Fn::Fn(&Compressed, ResponseBody.clone());
tracing::info!(/* ... */ CompressedLength = Final.Length(), /* ... */ "<<< COMPRESSED");
// ...
let body = Final.Bytes();
```

**Preserved** - the dev log still reports the serialised length and still reports it before the
response is built, and the fallback to the raw upstream body on a serialisation error still applies
to both the length and the body. The metric is still the compressed length, never the marker length -
that distinction is a documented reporting contract.

**Probe** - `grep -c 'serde_json::to_vec' crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Fn.rs`
must be 1. Then compare the dev log's `CompressedLength` with `body.len()` for one compressed
response: they must be equal.

### 7.5 Memory-bounded line sampling instead of collecting every line

**When** - a whole document is collected into a `Vec<&str>` only to index a window out of it, so a
10 MB payload costs 10 MB of pointers to serve 200 lines.

**Site** - `crates/aphrodite/Source/Retrieve/Page/Fn.rs:24`

**Before** -

```rust
let Limit = match Limit == 0 {
	true => usize::MAX,
	false => Limit.min(10_000),
};
let Lines:Vec<&str> = Content.lines().collect();
let Total = Lines.len();
```

**After** -

```rust
// crates/aphrodite/Source/Retrieve/Page/Fn.rs
let Total = Content.lines().count();
let Window:String = Content
	.lines()
	.skip(Offset)
	.take(Limit)
	.collect::<Vec<&str>>()
	.join("\n");
```

**Preserved** - `Total` is still the real line count (which is what the response reports as
`Truncated`'s basis), the window is still `Offset..Offset+Limit`, `Limit == 0` still means the whole
document, and an empty document is still a valid entry rather than an out-of-range offset. Two passes
over the lines replace one pass plus a full-length vector.

**Probe** - `grep -c 'Content.lines().collect()' crates/aphrodite/Source/Retrieve/Page/Fn.rs` must be 0. Then retrieve a 20,000-line entry with `offset:19990&limit:10` and with `limit:0`: the window must
be the last ten lines and the whole document respectively, with the same `Total` as before.

## Not applicable here

Each entry below is a transformation the design review proposes that this tree does not need, and the
measured count that settles it. They stay listed so the next reader knows the omission was checked.

- **Remove `.iter().cloned()` in favour of a consuming iterator.** The audit's ownership measurement
  records that `.iter().cloned()` appears nowhere in either crate (0 of 86 ownership-churn hits), so
  the only ownership work left is the 77 combinator-closure clones and 8 `.to_vec()` calls, which
  recipes 1.8, 4.8, 7.2, 7.3 and 7.4 cover.
- **Convert `match true` / `match false` to a predicate.** Of the 20 dummy scrutinees in both crates,
  all 20 are `match ()`; there is no `match true` or `match false` anywhere, so the fix is the single
  shape recipe 1.1 handles rather than a family.
- **Remove an `.await` from inside a synchronous combinator closure.** The audit measures 0 such
  sites: the only three async-in-loop shapes are ordinary `for` loops with an `.await` in the body,
  and recipe 2.3 is the one that applies to the retry loop among them.

## Claim-to-test

Every recipe's probe is runnable as written; this table is the net that says whether the catalogue as
a whole landed. "No output" means the command printed nothing and exited non-zero.

| Claim                                             | Probe                                                                                                                        | Pass condition                                                         |
| :------------------------------------------------ | :--------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------- |
| No recipe still has its before shape at its site  | for each `**Site**` path, `grep -n` the distinctive token named in its `**Probe**`                                           | every probe's stated count holds                                       |
| No dummy scrutinee survives the pass              | `grep -rn 'match ()' crates/aphrodite/Source crates/aphrodite-hermes/Source`                                                 | no output                                                              |
| No traversal ends with an artificial error        | `grep -rnE -e 'Err\(\(\)\)' -e 'Ok::<\(\), \(\)>' crates/aphrodite/Source crates/aphrodite-hermes/Source`                    | no output                                                              |
| No facade re-export came back                     | `grep -rn 'pub use' crates/aphrodite/Source crates/aphrodite-hermes/Source`                                                  | no output                                                              |
| No lock is held across a locking call             | `grep -rn -B3 '\.lock()' crates/aphrodite/Source crates/aphrodite-hermes/Source`                                             | no enclosing function holds a guard on a static while calling a locker |
| The three BEHAVIOUR recipes shipped separately    | `git log --oneline -- crates/aphrodite/Source/Preview/Builder/GitLog.rs crates/aphrodite/Source/PollWorker/Update/Fn.rs`     | three commits, none of which also carries a style-only recipe          |
| The C ABI contract is untouched                   | `python3 Maintain/check_ffi_contract.py`                                                                                     | `PASS`, 0 violations                                                   |
| The wire field names are untouched                | `git diff -- crates/aphrodite/Source/Proxy/Relay/Request/Struct.rs crates/aphrodite/Source/Proxy/CCR/Notification/Struct.rs` | no changed `#[serde(rename)]` or field name                            |
| Formatting is rustfmt-clean                       | `cargo fmt --all -- --check`                                                                                                 | exit 0                                                                 |
| Both crates still build                           | `cargo check -p aphrodite -p aphrodite-hermes --all-targets`                                                                 | exit 0                                                                 |
| Both crates still pass their tests                | `cargo test -p aphrodite` and `cargo test -p aphrodite-hermes`                                                               | exit 0, 0 failed                                                       |
| Error text is unchanged unless the recipe says so | `git diff -U0 -- crates` and read every changed `anyhow!`, `bail!`, `with_context` and `.context(` line                      | each changed message is named in the recipe that changed it            |
| No fallback runs after a winner                   | replace a lower-priority closure with `panic!()` and run `cargo test -p <package>`                                           | no test panics                                                         |
