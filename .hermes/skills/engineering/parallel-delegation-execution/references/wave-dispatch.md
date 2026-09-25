# Dispatch payloads and JSON serialization

- **Children know nothing of this conversation.** Embed the full contract in each task: repo absolute path, house style (tabs, license, author), the required docs/instruction files to read, the exact type/API shapes to build against, and the verification command with its expected output. Self-contained tasks do not drift. CLAIM.

- **Read the NEWEST planning/instruction file before executing a mass change.** Later directives supersede earlier ones and can reverse a rename or brand decision made earlier in the same session - a scripted mass rewrite is costly to undo.

## Wave structure

1. Parent does the shared-file foundation first (renames, manifests, .gitmodules, symlinks) and verifies it (cargo test / smoke probe green). Input: the pre-wave tree. Output: a shared foundation no child may write.
2. Wave 1: new modules, one child per file, each with the exact public-API contract it must expose (other children will import those names). Input: the verified foundation plus per-child ownership lists. Output: new module files with the promised API.
3. Parent integrates: run the workspace test once after the wave; fix cross-module mismatches yourself (children cannot see each other's code). Input: the wave's files. Output: a green workspace test run.
4. Wave N: dependent layers (CLI binary, tests, installers) - same rules. Input: the integrated tree. Output: the dependent-layer files.
5. Before declaring done, run the full verification ladder yourself and record results in the planning archive. Input: the final tree. Output: archive records plus the done state.

- **`delegate_task` tasks JSON can refuse to parse** ("Expecting ',' delimiter" / "Expecting property name" / "Invalid \\escape" -> "received a string that could not be parsed as JSON"). The failure is content-shape driven, not content-dependent. Fixes in order: (1) write the long brief into a repo file (gitignored scratch dir) and dispatch a SHORT context 'read <path> and follow it exactly'; (2) collapse to single-line prose and drop the `output_schema` - the load-bearing halves are the SHORT goal and the ABSENT `output_schema`. After TWO failed attempts with the same shape, switch to the file-pointer strategy - never a third dispatch on the same payload.

- **Goal text must contain no literal `{...}` brace literals.** The validator refuses the whole batch with "contains an unexpanded template marker - subagents cannot resolve placeholders" and fires on the braces themselves - even a literal `{model_id}` in prose blocks dispatch (a URL written as `{base_url}/chat/completions`, a JSON shape `{"model": id}`). The same rejection hits ANGLE-BRACKET placeholders: `gh run watch <RUN_ID>` or `gh release view <TAG>` block dispatch until you substitute the concrete value or rephrase in words. Phrase URLs, JSON bodies, and paths in words; substitute every placeholder before calling.

Pitfall (reproduction masking): on macOS, `/tmp` is a symlink to `/private/tmp` and ADDS a path level, so a "shallow path" / depth-index bug may not reproduce when staged under `/tmp` - stage at a genuinely shallow path (fewer than 4 parents, e.g. `/opt/<name>`), or compute the parents length directly to confirm the claim before declaring it reproduced or non-reproduced.
