#!/bin/bash

set -e

echo "🚀 Starting FNC CLI installation..."

# Create .local/bin if it doesn't exist
LOCAL_BIN="$HOME/.local/bin"
mkdir -p "$LOCAL_BIN"

# Check if running on macOS
OS=$(uname -s)
if [ "$OS" != "Darwin" ]; then
    echo "❌ This installer only supports macOS"
    exit 1
fi

# Get the latest release version from GitHub
echo "🔍 Fetching latest version..."
VERSION=$(curl -s https://api.github.com/repos/eulke/fnc/releases/latest | grep -o '"tag_name": ".*"' | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
    echo "❌ Failed to fetch latest version"
    exit 1
fi

# Determine architecture and set download URL
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    BINARY_URL="https://github.com/eulke/fnc/releases/download/${VERSION}/fnc-macos-arm64.tar.gz"
else
    BINARY_URL="https://github.com/eulke/fnc/releases/download/${VERSION}/fnc-macos-amd64.tar.gz"
fi

echo "📥 Downloading FNC CLI from: $BINARY_URL"
TEMP_DIR=$(mktemp -d)
TEMP_FILE="$TEMP_DIR/fnc.tar.gz"

# Download with curl and check HTTP status code
HTTP_RESPONSE=$(curl -L -w "%{http_code}" "$BINARY_URL" -o "$TEMP_FILE")
if [ "$HTTP_RESPONSE" != "200" ]; then
    echo "❌ Failed to download binary (HTTP $HTTP_RESPONSE)"
    echo "Response content:"
    cat "$TEMP_FILE"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Extract the tar.gz file
echo "📦 Extracting archive..."
tar -xzf "$TEMP_FILE" -C "$TEMP_DIR"

# Find the binary in the extracted contents
BINARY_PATH=$(find "$TEMP_DIR" -type f -name "fnc")
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Could not find fnc binary in the extracted archive"
    rm -rf "$TEMP_DIR"
    exit 1
fi

echo "📦 Installing FNC CLI..."
# Backup existing binary if it exists
if [ -f "$LOCAL_BIN/fnc" ]; then
    echo "💾 Backing up existing FNC CLI..."
    mv "$LOCAL_BIN/fnc" "$LOCAL_BIN/fnc.backup"
fi

# Install new binary
chmod +x "$BINARY_PATH"
mv "$BINARY_PATH" "$LOCAL_BIN/fnc"

# Clean up temporary directory
rm -rf "$TEMP_DIR"

# Verify the binary works
if ! "$LOCAL_BIN/fnc" --version >/dev/null 2>&1; then
    echo "❌ Installation failed - binary is not executable"
    if [ -f "$LOCAL_BIN/fnc.backup" ]; then
        echo "🔄 Restoring backup..."
        mv "$LOCAL_BIN/fnc.backup" "$LOCAL_BIN/fnc"
    fi
    exit 1
fi

# Update shell configuration
echo "🛠️  Updating shell configuration..."

update_rc() {
    local RC_FILE="$1"
    local EXPORT_PATH='export PATH="$HOME/.local/bin:$PATH"'
    
    echo "🔍 Checking $RC_FILE..."
    
    if [ ! -f "$RC_FILE" ]; then
        echo "📝 $RC_FILE does not exist, skipping..."
        return 1
    fi
    
    if grep -q "\.local/bin" "$RC_FILE"; then
        echo "✅ PATH already configured in $RC_FILE"
        return 1
    fi
    
    echo "📝 Adding PATH to $RC_FILE..."
    echo "" >> "$RC_FILE"  # Ensure newline before our addition
    echo "# Added by FNC CLI installer" >> "$RC_FILE"
    echo "$EXPORT_PATH" >> "$RC_FILE"
    echo "" >> "$RC_FILE"  # Ensure newline after our addition
    echo "✨ Successfully updated $RC_FILE"
    return 0
}

# Detect current shell and update appropriate RC file
CURRENT_SHELL=$(basename "$SHELL")
echo "🐚 Detected shell: $CURRENT_SHELL"

case "$CURRENT_SHELL" in
    "zsh")
        update_rc "$HOME/.zshrc"
        ;;
    "bash")
        update_rc "$HOME/.bashrc"
        ;;
    *)
        echo "⚠️ Unknown shell: $CURRENT_SHELL"
        if [ -f "$HOME/.zshrc" ]; then
            update_rc "$HOME/.zshrc"
        elif [ -f "$HOME/.bashrc" ]; then
            update_rc "$HOME/.bashrc"
        else
            echo "❌ No supported shell configuration file found"
            echo "Please add the following line to your shell configuration file:"
            echo 'export PATH="$HOME/.local/bin:$PATH"'
        fi
        ;;
esac

echo "✅ FNC CLI installation complete!"
echo "🎉 Run 'fnc --help' to get started"

# Cleanup backup if installation was successful
if [ -f "$LOCAL_BIN/fnc.backup" ]; then
    echo "🧹 Cleaning up backup..."
    rm "$LOCAL_BIN/fnc.backup"
fi
