# Grammar diff: proposed v1 vs actual current

The refactor policy (`rewrite.md` §CCR protocol needs versioning) proposes
`CCR:v1:<hash>:<content-type>:<byte-size>:<mode>:<preview>`. That grammar is
**NOT implemented**. Probes: `grep -rn "CCR:v1" crates/` returns nothing;
`grep -n proxy_format_ccr_output crates/aphrodite/src/proxy.rs` shows the
three-field template.

| Aspect     | Proposed v1 (rewrite.md) | Actual current (verified)             |
| ---------- | ------------------------ | ------------------------------------- |
| Version    | explicit `v1` field      | No version field in the marker        |
| Separators | colon `:`                | Pipe `\|`                             |
| Wrappers   | none                     | ASCII `<<<` ... `>>>`                 |
| Preview    | Inside the marker        | Outside, on preceding lines           |
| Mode       | Explicit field           | No mode field (internal routing only) |

## Observed type values (probe)

Observed type values in source/tests: `text`, `code_rust`, `terminal`,
`build`, `search`, `yaml`, `tool`. This list is an observation, not the
definition. The canonical type set is the classifier in
`crates/aphrodite/src/preview/builders/`; re-derive the set from that
classifier, never from this list, because this list drifts out of sync with
the classifier.
