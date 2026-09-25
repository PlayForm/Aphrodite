# Providers & Model Aliases

Use when the user wants to switch the model/provider the agent - and the
Aphrodite plugin running inside it - uses. Set via `hermes model` (picker) or
`hermes setup`. 35+ provider profiles ship as plugins under
`plugins/model-providers/`; user plugins of the same name override. Full
docs: https://hermes-agent.nousresearch.com/docs/integrations/providers

## Providers

| Provider                                                                                                                                      | Auth              | Key env var(s)                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------- |
| openrouter                                                                                                                                    | API key           | `OPENROUTER_API_KEY`                                                                           |
| anthropic                                                                                                                                     | API key           | `ANTHROPIC_API_KEY` (also `CLAUDE_CODE_OAUTH_TOKEN`)                                           |
| nous                                                                                                                                          | OAuth device code | `hermes auth add nous` (or `NOUS_API_KEY`)                                                     |
| openai-codex                                                                                                                                  | OAuth             | `hermes auth add openai-codex`                                                                 |
| qwen-oauth                                                                                                                                    | OAuth             | `hermes auth add qwen-oauth`                                                                   |
| minimax-oauth                                                                                                                                 | OAuth             | `hermes auth add minimax-oauth`                                                                |
| copilot                                                                                                                                       | Token             | `COPILOT_GITHUB_TOKEN` / `GH_TOKEN` (Copilot device flow - `gh auth login` tokens do NOT work) |
| copilot-acp                                                                                                                                   | External CLI      | Copilot CLI on PATH or `COPILOT_CLI_PATH`                                                      |
| gemini                                                                                                                                        | API key           | `GOOGLE_API_KEY` or `GEMINI_API_KEY`                                                           |
| xai                                                                                                                                           | API key           | `XAI_API_KEY` (SuperGrok OAuth also supported)                                                 |
| deepseek                                                                                                                                      | API key           | `DEEPSEEK_API_KEY`                                                                             |
| zai (GLM)                                                                                                                                     | API key           | `GLM_API_KEY` / `ZAI_API_KEY`                                                                  |
| minimax / minimax-cn                                                                                                                          | API key           | `MINIMAX_API_KEY` / `MINIMAX_CN_API_KEY`                                                       |
| kimi-coding / -cn                                                                                                                             | API key           | `KIMI_API_KEY` / `KIMI_CN_API_KEY`                                                             |
| alibaba (+coding-plan)                                                                                                                        | API key           | `DASHSCOPE_API_KEY` / `ALIBABA_CODING_PLAN_API_KEY`                                            |
| xiaomi                                                                                                                                        | API key           | `XIAOMI_API_KEY`                                                                               |
| huggingface                                                                                                                                   | Token             | `HF_TOKEN`                                                                                     |
| fireworks / novita / nvidia / deepinfra / gmi / arcee / stepfun / upstage / kilocode / ai-gateway / opencode-zen / opencode-go / ollama-cloud | API key           | `<NAME>_API_KEY`                                                                               |
| bedrock / vertex / azure-foundry                                                                                                              | Cloud SDK / key   | AWS SDK creds / Vertex ADC / `AZURE_FOUNDRY_API_KEY`                                           |
| custom                                                                                                                                        | Config            | `model.base_url` + `model.api_key` in config.yaml                                              |

Multiple credentials per provider pool and rotate automatically (`hermes
auth`). Fallback chain when the primary fails: `hermes fallback
add|remove|list`.

**Aphrodite note**: the CCR compression engine is upstream-agnostic - it
transforms tool output inside the Hermes host, independent of which provider
row is active, so switching `/model` mid-session does not affect the proxy.
What DOES affect the engine's proxy subprocesses is environment: make sure
`terminal.env_passthrough` includes `APHRODITE_API_KEY` (and `PATH`, `HOME`)
so the engine's proxy processes start with the key they need.

## User-defined model aliases

Work with `/model <name>` in CLI and every gateway platform. Resolved by
`hermes_cli/model_switch.py::resolve_alias()`; user aliases are checked BEFORE
the built-in table, so a user `sonnet`/`grok` shadows the built-in.

```yaml
# Full form
model_aliases:
    fav:
        model: claude-sonnet-4.6
        provider: anthropic
    local-qwen:
        model: qwen3.5:397b
        provider: custom
        base_url: "https://ollama.com/v1"
    theta:
        model: theta-1
        provider: custom
        base_url: "https://theta.example.com/v1"
        key_env: THETA_API_KEY # or: api_key: "${THETA_API_KEY}"

# Short form ("provider/model"), also via CLI:
#   hermes config set model.aliases.fav openrouter/anthropic/claude-sonnet-4.6
model:
    aliases:
        fav: openrouter/anthropic/claude-sonnet-4.6
```

`/model fav` - session-scoped; add `--global` to persist as default.

An alias with its own `base_url` authenticates with its own credential
(`api_key`, which also accepts a `"${VAR}"` reference, or `key_env`). With
neither set the key is resolved from the alias HOST, never carried over from
the provider that was active before the switch.

Built-in aliases (catalog-resolved against the active provider): `sonnet`,
`opus`, `haiku`, `claude`, `gpt5`, `gpt`, `codex`, `o3`, `o4`, `gemini`,
`deepseek`, `grok`, `llama`, `qwen`, `minimax`, `nemotron`, `kimi`, `glm`,
`step`, `mimo`, `trinity`.

**Stop if** a proxy failure is blamed on the provider - `aphrodite_stats`
and `aphrodite_rebuild` report proxy health; a missing `APHRODITE_API_KEY`
in `env_passthrough` is a Hermes config issue, not a provider issue.

**Recovery** - permitted: set `terminal.env_passthrough` explicitly and
restart the session. Prohibited: hardcoding provider keys into
`~/.hermes/aphrodite/aphrodite.toml` or plugin source.

## Verification

| Claim                           | Test                                         | Pass condition                |
| ------------------------------- | -------------------------------------------- | ----------------------------- |
| Alias resolves                  | `/model fav` in the dev profile              | Session model == alias target |
| Key never leaks into engine cfg | Inspect `~/.hermes/aphrodite/aphrodite.toml` | No API keys present           |
| Proxy healthy under switch      | Switch `/model`, then `aphrodite_stats`      | Proxy health stays green      |
