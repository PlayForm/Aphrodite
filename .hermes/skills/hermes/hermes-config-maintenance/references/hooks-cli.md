# Hooks CLI workflow - pruning hooks from config

Hooks are declared under the `hooks:` key and are removed the same way as any other config block - via `hermes config`, never by editing the file:

```bash
hermes config unset hooks.pre_tool_call # drop an event block entirely
hermes config unset hooks.pre_api_request
hermes config set hooks.post_tool_call '[{"command": "~/.hermes/agent-hooks/x.sh", "matcher": "write_file|patch", "timeout": 5}]' # replace the list
hermes hooks list                                                                                                                 # verify: only survivors appear
```

## Pitfalls

- `hermes hooks revoke <command>` only clears the ALLOWLIST consent entry - the hook stays in `config.yaml` and still fires; removal is `hermes config unset/set` on the hooks key.
- Orphaned hook scripts left in `~/.hermes/agent-hooks/` are inert but confuse audits - delete them after unwiring.
- A newly-wired hook shows `✗ not allowlisted` until approved; `hooks_auto_accept: true` lets it run anyway; `hermes hooks doctor` re-validates scripts whose approval mtime went stale after a copy/replace ("script modified since approval").
- The Aphrodite plugin's own hooks (`on_session_start`, `transform_tool_result`, `pre_llm_call`, `transform_terminal_output`, `post_llm_call`) are declared inside the plugin, NOT under `hooks:` - never prune plugin-internal hooks from config.yaml.

## Claim-to-test

| Claim | Test | Pass condition |
| --- | --- | --- |
| `hermes config unset` removes the hook | Run `hermes config unset hooks.pre_tool_call`, then `hermes config get hooks.pre_tool_call` | Key absent; YAML parses |
| `hermes hooks revoke` does not remove the hook | Run `hermes hooks revoke <command>`, then `hermes config get hooks.<key>` | Hook still present in `config.yaml` |
| Survivors list is accurate | `hermes hooks list` after unset/set | Only remaining hooks appear |