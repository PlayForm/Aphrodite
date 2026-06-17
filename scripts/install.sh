#!/bin/bash
# aphrodite install script — downloads latest release binary
set -euo pipefail

REPO="PlayForm/Aphrodite"
VERSION="${1:-latest}"
INSTALL_DIR="${HOME}/.hermes/aphrodite"
BIN="$INSTALL_DIR/aphrodite"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
	arm64 | aarch64) ARCH="arm64" ;;
	x86_64 | amd64) ARCH="x64" ;;
	*)
		echo "Unsupported architecture: $ARCH"
		exit 1
		;;
esac

echo "Installing aphrodite $VERSION for $OS-$ARCH..."

# Download
URL="https://github.com/$REPO/releases/download/$VERSION/aphrodite-$OS-$ARCH"
mkdir -p "$INSTALL_DIR"
curl -fsSL "$URL" -o "$BIN" || {
	echo "Download failed. Trying release binary..."
	curl -fsSL "https://github.com/$REPO/releases/latest/download/aphrodite" -o "$BIN" || {
		echo "Failed to download. Build from source: cargo install aphrodite"
		exit 1
	}
}
chmod +x "$BIN"

# Create default config
if [ ! -f "$INSTALL_DIR/aphrodite.toml" ]; then
	cat > "$INSTALL_DIR/aphrodite.toml" << 'EOF'
[defaults]
api_url = "https://api.example.com"
model = "gpt-4o"

[[proxies]]
name = "cache"
listen = "127.0.0.1:9797"
mode = "cache"

[[proxies]]
name = "token"
listen = "127.0.0.1:9798"
mode = "token"
tool_relay = true
EOF
	echo "Default config created at $INSTALL_DIR/aphrodite.toml"
fi

echo ""
echo "✓ aphrodite $VERSION installed to $BIN"
echo "  Run:    $BIN"
echo "  Config: $INSTALL_DIR/aphrodite.toml"
echo "  Env:    export APHRODITE_API_KEY=sk-..."
