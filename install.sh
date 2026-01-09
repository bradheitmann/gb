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

# Copy agents directory to ~/.config/gb/agents for global access
echo "📁 Installing agent personas globally..."
mkdir -p ~/.config/gb
cp -r ./agents ~/.config/gb/
echo "✅ Agents installed to ~/.config/gb/agents"

# Copy config if it doesn't exist globally
if [ -f "config.toml" ] && [ ! -f ~/.config/g3/config.toml ]; then
    echo "📝 Installing config globally..."
    mkdir -p ~/.config/g3
    cp config.toml ~/.config/g3/config.toml
    echo "✅ Config installed to ~/.config/g3/config.toml"
elif [ ! -f "config.toml" ]; then
    echo "⚠️  Warning: No config.toml found. Copy config.example.toml to config.toml and add your API key."
    echo "   Then run: cp config.toml ~/.config/g3/config.toml"
fi

echo ""
echo "✅ GB installed successfully!"
echo ""
echo "Try these commands (from any directory):"
echo "  gb --help"
echo "  gb"
echo "  gb --agent daria \"review my code\""
echo ""
echo "👑 On Wednesdays, we ship pink code. 💅✨"
