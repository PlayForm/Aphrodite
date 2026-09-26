# Mode decision matrix (source vs installed)

Worked matrix consumed by Step 1 of `aphrodite-development` (SKILL.md). The
matrix is the evidence order; the gate commands in SKILL.md Step 1 are the
probe that fills it in.

| Check           | Source development                          | Installed/user diagnosis                                                               |
| --------------- | ------------------------------------------- | -------------------------------------------------------------------------------------- |
| Workspace       | Parent Cargo workspace must exist           | Cargo workspace may legitimately be absent                                             |
| Plugin source   | Loader source in `plugins/aphrodite/`       | Installed hooks-only loader (`~/.hermes/plugins/aphrodite`: plugin.yaml + **init**.py) |
| Binary          | Build from source allowed                   | Release artifact download path used                                                    |
| Code edits      | Local source reload/restart required        | Do not assume edits affect installed plugin                                            |
| Version truth   | Workspace manifests plus submodule metadata | Installed binary plus `BINARY_VERSION` pairing                                         |
| Primary failure | Stale process/loader                        | Missing, incompatible, or unavailable artifact                                         |

The two modes fail independently. A source workspace can be present while the
installed runtime home is absent, and the reverse. Each check reports its own
exit code, so the mode decision is never a single boolean: `echo "WORKSPACE:$?"`
and `echo "RUNTIME_HOME:$?"` each print their own result. A row is a finished
observation only after its check ran.

Current pins (verify against the manifests, never memorize): binary **1.6.2**
(`crates/aphrodite` + `crates/aphrodite-hermes`), plugin **2.2.2**
(`plugins/aphrodite/plugin.yaml`), `BINARY_VERSION` **1.6.2**
(`~/.hermes/aphrodite/BINARY_VERSION`). The pin that matters for the running
plugin is `BINARY_VERSION`: `grep -q '^1.6.2$' ~/.hermes/aphrodite/BINARY_VERSION`
is the check that pairs the installed binary with the pin.

Mode consequences that the SKILL.md gates enforce:

- Source mode: `cargo build` is permitted, and a build must be followed by
  reinstall + restart (`aphrodite setup` from `target/release`, then a fresh
  Pane 1 process).
- Installed mode: never `cargo build`; the workspace is not supposed to be
  repaired, and source edits are not assumed to reach the installed plugin.
- Both checks non-zero: not an Aphrodite dev environment; stop before any
  advice.
