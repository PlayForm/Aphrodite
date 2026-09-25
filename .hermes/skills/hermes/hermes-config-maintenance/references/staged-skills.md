# Staged skill edits - applying ~/.hermes/pending/skills/

When `skills.write_approval` was on, `skill_manage` batches land as JSON files at `~/.hermes/pending/skills/<id>.json` instead of being applied. The `hermes skills` CLI has NO approve/pending subcommand - `/skills pending` is TUI-only, and `/skills approve all` from another session does NOT reliably apply them. Verify the target skill file actually contains the update before trusting an approval.

## Apply procedure (from an agent session)

1. Each JSON has `payload.operations[]` - plain patch ops with exact `old_string`/`new_string` and an optional `file_path` (references/...).
2. Resolve the skill name to `~/.hermes/skills/<category>/<name>/SKILL.md` (or `<name>/<file_path>`); apply each op with the patch tool using the exact strings from the JSON.
3. SKIP ops whose `old_string` no longer matches the file - content already applied or superseded; applying them fails or corrupts.
4. Delete the JSON files afterwards (`rm ~/.hermes/pending/skills/*.json`) - that clears the 'N pending' banner in other sessions.

## Claim-to-test

| Claim | Test | Pass condition |
| --- | --- | --- |
| Approval applies the queue | Read the target SKILL.md after approving | The patch's new_string is present in the file |
| Queue cleared | `ls ~/.hermes/pending/skills/` | No .json files remain |