#!/bin/sh
# Gitu installer script
set -e

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        EXT="tar.gz"
        ;;
    darwin)
        case "$ARCH" in
            x86_64) TARGET="x86_64-apple-darwin" ;;
            arm64) TARGET="aarch64-apple-darwin" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        EXT="tar.gz"
        ;;
    mingw*|msys*|cygwin*)
        TARGET="x86_64-pc-windows-msvc"
        EXT="zip"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

REPO="FizzWizzleDazzle/gitu"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="gitu"

echo "Installing gitu for $TARGET..."

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Download and extract
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/gitu-${TARGET}.${EXT}"
echo "Downloading from $DOWNLOAD_URL..."

if [ "$EXT" = "tar.gz" ]; then
    curl -sL "$DOWNLOAD_URL" | tar xz -C "$INSTALL_DIR"
else
    TEMP_ZIP="/tmp/gitu-${TARGET}.zip"
    curl -sL "$DOWNLOAD_URL" -o "$TEMP_ZIP"
    unzip -q -o "$TEMP_ZIP" -d "$INSTALL_DIR"
    rm "$TEMP_ZIP"
fi

# Make executable
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "✓ gitu installed to $INSTALL_DIR/$BINARY_NAME"
echo ""

# Check if in PATH
if echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "You can now run: gitu"
else
    echo "Add to your PATH by adding this line to your shell profile:"
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
fi
