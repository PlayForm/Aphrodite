# Documentation linting (L-1..L-10)

A `.hermes/**/*.md` document is compliant only when all of these hold:

- L-1: Exactly one `status` field.
- L-2: Every `deprecated`/`archived` document names a successor.
- L-3: No active skill references a deprecated skill without a "Historical only - do not execute" label.
- L-4: Every mutation command appears in a step with `Preconditions`, `Verify`, `Stop if`, and `Recovery`.
- L-5: Every hard-coded source line is marked historical or paired with a path/function search.
- L-6: Every threshold is labeled live-read, default, or test fixture.
- L-7: Every environment variable has defined consumer status: active, inactive, or removed.
- L-8: Every mention of a branch has declared permitted branch scope.
- L-9: Every release action has a human-approval boundary.
- L-10: No secret-like variable is printed in a sample command.

All `.hermes/**/*.md` stay prettier-clean (`npx prettier --check .hermes/**/*.md`; tabs, width 100, proseWrap preserve).
