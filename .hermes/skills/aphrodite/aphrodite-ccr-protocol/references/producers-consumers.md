# Producers and consumers

| Role            | Component                                        | Verified location                                   |
| --------------- | ------------------------------------------------ | --------------------------------------------------- |
| Proxy producer  | `smart_marker` (token mode), `cache_marker`      | `crates/aphrodite/src/proxy.rs:1981-1989`           |
| Hook producer   | Dylib transform path (segmented markers)         | `crates/aphrodite-hermes/src/lib.rs`                |
| Parser          | `find_markers` / `parse_marker_hash`             | `crates/aphrodite/src/resolve/parse.rs`             |
| Hash extraction | `extract_hashes` (HASH_RE, 3 delimiter families) | `crates/aphrodite/src/marker/parse.rs`              |
| Resolver        | `resolve_one` + `resolve_recursive`              | `crates/aphrodite/src/resolve/{one,recursive}.rs`   |
| Retrieve tool   | `aphrodite_retrieve` (exact hash match)          | `crates/aphrodite-hermes/src/lib.rs` tools dispatch |

Probes: `grep -n "smart_marker" crates/aphrodite/src/proxy.rs`;
`grep -n "find_markers" crates/aphrodite/src/resolve/parse.rs`;
`grep -n "extract_hashes" crates/aphrodite/src/marker/parse.rs`.

Hermes itself is protocol-agnostic: no `CCR:` reference exists in the Hermes
agent codebase. All marker logic lives in the Rust crates and the plugin
shim. Probe: `grep -rn "CCR:" .` from the Hermes agent repo root returns no
match.
