# Aphrodite Development Context

## Project
- Rust proxy for LLM API with CCR compression
- Single crate: crates/aphrodite
- Plugin: plugins/aphrodite (10 modules, v1.61.0)
- Config: aphrodite.toml
- Proxy binary: ~/.hermes/aphrodite/aphrodite (v0.5.54)

## Dev Flow
- cargo watch -x 'run -p aphrodite' in pane 0
- hermes --profile aphrodite-compress-aggressive in pane 1
- Test via MCP WezTerm, never interrupt main session

## Key Paths
- Binary: target/release/aphrodite
- Plugin: plugins/aphrodite/ (10 files)
- Env: ~/.hermes/.env (APHRODITE_API_KEY)
- Plans: .hermes/plans/ (0-1-2 numbered)
- Tasks: .hermes/tasks/ (1-5 numbered)
- Master: .hermes/MASTER-TASKS.md

## Agents
- Main: development + testing
- Background: research, code review, test generation
- Cron: health checks, release builds

## Linting
- ruff: cd plugins/aphrodite && ruff check .
- pyright: plugins/aphrodite/pyrightconfig.json
- cargo: cargo test -p aphrodite

## Profiles (7)
- aphrodite-barebone, aphrodite-proxy-cache, aphrodite-proxy-token
- aphrodite-compress-off, aphrodite-compress-light, aphrodite-compress-medium, aphrodite-compress-aggressive
