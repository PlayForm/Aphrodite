# CCR Tool Test Cases (adapted from the hook test cases)

Each active CCR tool needs at least the cases in the table. The application
column names the exact invocation to run. The pass condition names the exact
result that would prove the case. Any other result is a tool-contract failure;
it is not a reason to adjust the test.

| Case                         | CCR-tool application                                           | Pass condition                                                                                        |
| ---------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Valid nominal invocation     | `aphrodite_test(mode="quick")`; compress -> retrieve roundtrip | `status="ok"`; resolved bytes equal source normalized content                                         |
| Unknown extra keyword        | `aphrodite_retrieve(hash=..., bogus=1)`                        | Readable error or schema rejection; no crash, no marker loss                                          |
| Missing optional keyword     | `aphrodite_retrieve(query="x")` with no hash/path              | `{found: false, error}` - proxy rejects hash-less retrieve                                            |
| Empty text input             | `aphrodite_compress(content="")`                               | Defined result (valid empty entry or readable error); never a crash                                   |
| Large payload                | content above the live threshold                               | One valid marker; retrieve returns original bytes; preview truthful                                   |
| Error status / non-zero exit | retrieve unknown hash; proxy error                             | `{found: false, error}` surfaced, never swallowed                                                     |
| Handler exception            | dispatch error inside the tool relay                           | Readable diagnostic; fail-open default (original content preserved)                                   |
| Multiple registered handlers | N/A for tools (single-name dispatch) - applies to hooks        | Covered by `aphrodite-hook-reference`                                                                 |
| Registered but never invoked | Compare `plugin.yaml` tool list vs the live tool catalog       | Every registered tool invocable; orphans flagged                                                      |
| Restarted process            | Fresh Hermes session after a dylib change                      | Version handshake matches `BINARY_VERSION`; stale dylib excluded; store re-verified via stats/catalog |

The restarted-process case passes only when the version handshake matches
`BINARY_VERSION`; a stale dylib fails it. The store check after a restart uses
`stats` / `catalog`, not a re-read of the store file.
