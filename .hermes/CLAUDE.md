# aphrodite Project Rules

1. Never interrupt main session for testing — use MCP pane 1
2. Commit after every feature, push to PlayForm/Aphrodite
3. Kill :8787 proxy when output compression interferes
4. Always set APHRODITE_API_KEY before cargo watch
5. Tests must pass (6/6) before commit
6. Use single cargo watch -x 'run -p aphrodite' for dev
