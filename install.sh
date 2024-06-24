#!/bin/bash

# Determine the chip architecture
if [ "$(uname -m)" == "arm64" ]; then
    ARCH="arm64"
    URL="https://github.com/eulke/fnc/releases/download/v0.0.18/fnc-macos-arm64.tar.gz"
else
    ARCH="amd64"
    URL="https://github.com/eulke/fnc/releases/download/v0.0.18/fnc-macos-amd64.tar.gz"
fi

echo "Detected architecture: $ARCH"
echo "Downloading from: $URL"

# Download the tarball
curl -L -O $URL

# Extract the tarball
tar -xzf fnc-macos-$ARCH.tar.gz

# Create ~/.local/bin if it doesn't exist
mkdir -p ~/.local/bin

# Move the binary to ~/.local/bin
mv fnc ~/.local/bin/

# Add ~/.local/bin to PATH
export PATH="$HOME/.local/bin:$PATH"

# Detect shell configuration file
if [ -f "$HOME/.zshrc" ]; then
    SHELL_CONFIG="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
    SHELL_CONFIG="$HOME/.bashrc"
else
    echo "Warning: Could not find a .zshrc or .bashrc file."
    echo "You may need to add ~/.local/bin to your PATH manually."
    SHELL_CONFIG=""
fi

# Add export command to shell configuration file
if [ -n "$SHELL_CONFIG" ]; then
    # Check if the export command already exists in the file
    if grep -q "export PATH=\"\$HOME/.local/bin:\$PATH\"" "$SHELL_CONFIG"; then
        echo "The export command already exists in $SHELL_CONFIG."
    else
        echo "Adding the export command to $SHELL_CONFIG..."
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$SHELL_CONFIG"
    fi
fi

# Clean up the temporary files
rm fnc-macos-$ARCH.tar.gz

echo "Installation complete. Verifying installation..."

# Verify installation
which fnc > /dev/null
if [ $? -eq 0 ]; then
    echo "Binary 'fnc' successfully installed and available in PATH."
else
    echo "Binary 'fnc' not found in PATH. You may need to add ~/.local/bin to your PATH."
fi
