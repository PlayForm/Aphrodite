# Hook Test Case Matrix

The 10-case matrix for `aphrodite-hook-contracts`. Run every case for every active hook; `transform_terminal_output` additionally requires the sentinel-output test. Expected outcomes come from the per-hook return contracts and the global contract (fail-open default for transform/lifecycle hooks; `pre_tool_call` fail-closed).

## The 10 cases

| #   | Case                         | Fixture                                                                     | Expected outcome                                                                                                                                          | Failure response                                             |
| --- | ---------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| 1   | Valid nominal invocation     | Invoke the hook with all documented keyword names and normal values         | Handler runs; the return follows the hook's return contract (pass-through or replacement)                                                                 | Fix handler parameters                                       |
| 2   | Unknown extra keyword        | Invoke with documented kwargs plus an extra field (e.g. `future_field="x"`) | Handler accepts the field via `**kwargs`; no exception                                                                                                    | Add `**kwargs` to the handler signature                      |
| 3   | Missing optional keyword     | Invoke without an optional field (e.g. no `task_id`)                        | Handler uses the documented default; no KeyError                                                                                                          | Give every field a default                                   |
| 4   | Empty text input             | Invoke with `output=""` (or `result=""`, `user_message=None`)               | Handler passes through; it must not return a destructive empty replacement                                                                                | Fix the pass-through path                                    |
| 5   | Large payload                | Invoke with output at or above the compression threshold                    | Replacement path honors the threshold and marker behavior; no CCR marker corruption                                                                       | Re-derive the threshold from the compression-safety contract |
| 6   | Error status / non-zero exit | Invoke with `status="error"` or `returncode=1`                              | The hook still fires; output is preserved unless a replacement is intended                                                                                | Remove early-return guards                                   |
| 7   | Handler exception            | Handler raises                                                              | Transform/lifecycle hooks fail open (log a structured error, original content used); `pre_tool_call` fails closed (tool blocked with the timeout message) | Fix handler error handling / boundedness                     |
| 8   | Multiple registered handlers | Register two handlers                                                       | The first non-None string wins for transform hooks; the first valid dict wins for `pre_tool_call`                                                         | Fix handler ordering logic                                   |
| 9   | Registered-but-never-invoked | Register a name with no production `invoke_hook` site                       | The callback never fires; the dead registration is detected                                                                                               | Fix the registration name or remove the dead hook            |
| 10  | Restarted process            | Restart Hermes and re-trigger the hook                                      | The hook is still registered and fires                                                                                                                    | Re-check the loader registration path                        |

## Sentinel-output test (mandatory for transform_terminal_output)

The known `stdout`-vs-`output` mismatch yields an empty replacement despite a successful-looking invocation. Guard it permanently:

1. Invoke the handler with a sentinel output (`output="SENTINEL_XYZ"`) and a non-zero `returncode`.
2. Assert the handler observed the sentinel. A handler reading `stdout` gets its `""` default and returns `""` - the failure shape.
3. Assert the pass-through path returns the exact sentinel bytes.

## Hook-specific assertions

The per-hook verification enumerations from the contract, kept as required assertions:

- **on_session_start** - pass-through (return ignored), handler exception (Hermes logs a warning and continues), unknown-kwarg, registered-but-never-invoked.
- **pre_tool_call** - modify-merge (args changed before dispatch), block (tool result is the message), approve (approval gate), non-dict ignored, handler exception / timeout (tool blocked, message visible), unknown-kwarg.
- **transform_tool_result** - pass-through, replacement, empty-result, handler exception (fail-open), unknown-kwarg, error-status (non-zero / error result still reaches the hook), large payload (threshold + marker behavior).
- **transform_terminal_output** - pass-through, replacement, empty-result, exception, unknown-kwarg, non-zero-exit (`returncode != 0` must not erase output), sentinel-output test.
- **pre_llm_call** - context injection (string and `{"context": ...}` forms), empty/no-op, handler exception, unknown-kwarg, copy-discard test (mutate `conversation_history` in place; assert the live transcript is unchanged).
- **post_llm_call** - pass-through, handler exception (swallowed by `_invoke_hook_safely`), unknown-kwarg.
