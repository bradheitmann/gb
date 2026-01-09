#!/bin/bash
# GB Installation Script
# Handles VisionBridge dylib properly for cargo install

set -e

echo "✨💖 Installing GB (G3-Glitter-Bomb) 💖✨"
echo ""

# Build in release mode
echo "🔨 Building GB in release mode..."
cargo build --release

# Install the binary
echo "📦 Installing gb to ~/.cargo/bin..."
cargo install --path . --force

# Copy the VisionBridge dylib
if [ "$(uname)" = "Darwin" ]; then
    echo "🍎 Copying VisionBridge.dylib for macOS..."
    if [ -f "./target/release/libVisionBridge.dylib" ]; then
        cp ./target/release/libVisionBridge.dylib ~/.cargo/bin/
        echo "✅ Copied libVisionBridge.dylib to ~/.cargo/bin/"
    else
        echo "⚠️  Warning: libVisionBridge.dylib not found"
        echo "   Computer control features may not work"
    fi
fi

echo ""
echo "✅ GB installed successfully!"
echo ""
echo "Try these commands:"
echo "  gb --help"
echo "  gb"
echo "  gb --agent daria \"review my code\""
echo ""
echo "👑 On Wednesdays, we ship pink code. 💅✨"
