# Runtime behavioral probes (Step 6 detail)

This file backs the Step 6 mandatory runtime list in SKILL.md. The list in
SKILL.md is the gate; each section here is one probe's procedure and pass
condition. Run all nine after ANY dependency upgrade, in the order below.

## 1. Version path before config loading

Run `aphrodite --version` with no `aphrodite.toml` present, then run it
with an existing config. Both runs must print the version and exit 0.
A non-zero exit means the early argument check in `main()` runs after
config loading; that is a CLI-startup breakpoint.

## 2. Startup with an existing configuration file

Start the proxy with a real `aphrodite.toml`. Check engine health with the
`aphrodite_stats` equivalent, then perform ONE compress/retrieve round trip
through the proxy. Both must succeed.

## 3. HTTP route registration, including catch-all/fallback

Start the proxy, request a real route, and request an unmatched path. The
catch-all/fallback must behave; the process must not panic with
"Path segments must not start with *".

## 4. WebSocket ping/pong, text, and binary

Send ping/pong, text, and binary messages through the proxy
(tokio-tungstenite message types). Payloads must arrive byte-identical.

## 5. Python/FFI init and thread interaction

Run `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`
and the FFI contract checker (`test_check_ffi_contract.py`). When pyo3
changed, also run a GIL-release/thread probe.

## 6. Marker/hash formatting and checksums

Compare marker hex output against Python `hashlib.sha256`; the bytes must
be identical (smart_crusher `_hash_field_name` parity).

## 7. Plugin import/load

Import the plugin in a fresh Hermes session. Never use a stale
dylib/process, because a stale one loads the previous binary and the probe
then tests the old build. Run the drift guard: `diff -q
plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`;
identical output is the guard's pass.

## 8. Release builds

Run `cargo build --release -p aphrodite -p aphrodite-hermes`, plus
per-platform builds when available.

## 9. Reinstall after a rebuild

Source the environment file first; its `cargo()` wrapper syncs binary +
dylib. A fresh release build lands in `target/release`. Run `aphrodite
setup` from that binary to install it.

## Failure classification matrix (Step 4 detail)

Recognition strings and default actions for Step 4 of SKILL.md. Worked
examples also live in `references/breakpoints.md`.

| Class                  | Recognition                                                                               | Default action                |
| :--------------------- | :---------------------------------------------------------------------------------------- | :---------------------------- |
| API rename             | symbol/type moved or renamed (e.g. `Message` variants, `DocumentMut`)                     | migrate source                |
| Trait-bound change     | `trait bound ... not satisfied` (e.g. `LowerHex`, axum `Handler`, tracing `DisplayValue`) | migrate source or pin locally |
| Feature rename         | `dep does not have that feature` (e.g. reqwest `rustls-tls` → `rustls`)                   | migrate feature string        |
| Semver incompatibility | behavior/API removed across major bump without replacement                                | pin locally or revert         |
| Runtime route behavior | starts but routes/fallback/WS misbehave                                                   | pin + behavioral tests        |
| ABI/FFI change         | pyo3/FFI surface change (e.g. `allow_threads` removed)                                    | pin; FFI contract gates       |
| Security advisory      | `cargo audit`/advisory for the target version                                             | pin to patched minor or defer |

## Verify block

The same block appears in SKILL.md Step 6:

```sh
aphrodite --version
echo "EXIT:$?"
aphrodite_stats 2> /dev/null | head -5
echo "EXIT:$?"
python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py
echo "EXIT:$?"
```

All nine items must pass with recorded output. A compile pass is not
evidence for any of them.
