#!/usr/bin/env bash
set -euo pipefail
# aphrodite - minimal one-command local install
# curl -sSL https://raw.githubusercontent.com/PlayForm/Aphrodite/Current/Maintain/install.sh | bash
#
# Installs from a local clone or release build.  Expects:
#   target/release/aphrodite      (the Rust binary)
#   plugins/aphrodite/             (the Hermes plugin)
#   profiles/*/                    (7 profile directories)

REPO="${REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
HERMES="${HERMES:-$HOME/.hermes}"
BINARY="${HERMES}/aphrodite/aphrodite"
PLUGIN_SRC="${REPO}/plugins/aphrodite"
SKILLS_SRC="${PLUGIN_SRC}/skills"

echo "=== aphrodite install ==="
echo "  repo:   $REPO"
echo "  hermes: $HERMES"

# --- 1. Binary ---------------------------------------------------------------
mkdir -p "$(dirname "$BINARY")"
if [ -f "$REPO/target/release/aphrodite" ]; then
	cp "$REPO/target/release/aphrodite" "$BINARY"
	chmod +x "$BINARY"
	echo "  binary: $BINARY ($(du -h "$BINARY" | cut -f1))"
else
	echo "  binary: SKIP - no release build at target/release/aphrodite"
	echo "          Run: cargo build --release -p aphrodite"
fi

# --- 2. Plugin symlink -------------------------------------------------------
mkdir -p "$HERMES/plugins"
rm -rf "$HERMES/plugins/aphrodite"
ln -sf "$PLUGIN_SRC" "$HERMES/plugins/aphrodite"
echo "  plugin: $HERMES/plugins/aphrodite → $PLUGIN_SRC"

# --- 3. Skills (hermes namespace) --------------------------------------------
rm -rf "$HERMES/skills/hermes"
ln -sf "$SKILLS_SRC" "$HERMES/skills/hermes"
echo "  skills: $HERMES/skills/hermes → $SKILLS_SRC"

# --- 4. Profiles -------------------------------------------------------------
# 7 pre-configured profiles ship inside the repo under profiles/.
# Rather than recreating them from scratch, symlink the whole directory
# so config.yaml + any state-driven cache/log stays inside the repo.
mkdir -p "$HERMES/profiles"

PROFILE_NAMES=(
	barebone
	proxy-cache
	proxy-token
	compress-off
	compress-light
	compress-medium
	compress-aggressive
)

for name in "${PROFILE_NAMES[@]}"; do
	profile="aphrodite-$name"
	src="$REPO/profiles/$profile"
	dst="$HERMES/profiles/$profile"

	if [ ! -d "$src" ]; then
		echo "  profile: $profile - SKIP (no directory at $src)"
		continue
	fi

	# Remove stale symlink or directory, then re-link
	rm -rf "$dst"
	ln -sf "$src" "$dst"

	# Ensure the plugin is listed in the profile's config
	hermes plugins enable aphrodite --profile "$profile" 2>/dev/null || true

	echo "  profile: $profile ✓"
done

# Also enable in the default (active) profile
hermes plugins enable aphrodite 2>/dev/null || true

echo ""
echo "=== done ==="
echo "  Launch: hermes --profile aphrodite-compress-aggressive"
echo "  Proxy:  hermes --profile aphrodite-proxy-token"
echo "  Debug:  APHRODITE_DEBUG=1 hermes --profile aphrodite-compress-aggressive"
