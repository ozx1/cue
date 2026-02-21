#!/bin/bash
set -e

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

if [ "$ARCH" = "x86_64" ]; then
    ARCH="x86_64"
elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
    ARCH="arm64"
else
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

# Map OS and arch to binary name
case "$OS" in
    linux)
        if [ "$ARCH" = "arm64" ]; then
            BINARY="cue-linux-arm64"
        else
            BINARY="cue-linux-x86_64"
        fi
        ;;
    darwin)
        if [ "$ARCH" = "arm64" ]; then
            BINARY="cue-macos-arm64"
        else
            BINARY="cue-macos-x86_64"
        fi
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Installing cue for $OS ($ARCH)..."

# Get latest release tag
LATEST_RELEASE=$(curl -s https://api.github.com/repos/ozx1/cue/releases/latest | grep "tag_name" | cut -d '"' -f 4)

if [ -z "$LATEST_RELEASE" ]; then
    echo "Failed to fetch latest release"
    exit 1
fi

DOWNLOAD_URL="https://github.com/ozx1/cue/releases/download/$LATEST_RELEASE/$BINARY"

echo "Downloading $LATEST_RELEASE from GitHub..."
curl -L -o /tmp/cue "$DOWNLOAD_URL"

# Make executable
chmod +x /tmp/cue

# Install to /usr/local/bin
echo "Installing to /usr/local/bin (may require sudo)..."
sudo mv /tmp/cue /usr/local/bin/cue

echo ""
echo "✓ cue installed successfully!"
echo "Run 'cue -h' to get started"
