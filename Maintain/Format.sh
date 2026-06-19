#!/bin/sh
# Aphrodite Format - Prettier pass for Markdown, JSON, YAML
# TOML handled by taplo; Python by ruff; Rust by rustfmt
# Usage: sh Maintain/Format.sh

set -e

cd "$(dirname "$0")/.."

echo "→ Installing dependencies…"
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

echo "→ Running Prettier (md, json, yaml)…"
pnpm exec prettier --write \
	--ignore-path .prettierignore \
	"**/*.md" \
	"**/*.json" \
	"**/*.yml" \
	"**/*.yaml" \
	"*.md" \
	"*.json" \
	"*.yml" \
	"*.yaml"

echo "→ Format complete."
