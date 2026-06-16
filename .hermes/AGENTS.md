# Aphrodite Development Context

## Project
- Rust proxy for LLM API with CCR compression
- Single crate: crates/aphrodite
- Plugin: plugins/aphrodite
- Config: aphrodite.toml

## Dev Flow
- cargo watch -x 'run -p aphrodite' in pane 0
- hermes --provider aphrodite-token in pane 1
- Test via MCP WezTerm, never interrupt main session

## Key Paths
- Binary: target/debug/aphrodite
- Plugin: ~/.hermes/plugins/aphrodite
- Env: ~/.hermes/.env (APHRODITE_API_KEY)

## Agents
- Main: development + testing
- Background: research, code review, test generation
- Cron: health checks, release builds
