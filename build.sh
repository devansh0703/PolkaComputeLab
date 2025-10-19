#!/bin/bash

set -e

echo "🚀 Building PolkaComputeLab..."
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check if wasm target is installed
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "📦 Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

echo "🔨 Building runtime..."
cargo build --release -p polkacomputelab-runtime

echo ""
echo "🔨 Building node..."
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "📍 Binary location: ./target/release/polkacomputelab-node"
echo ""
echo "To run the node:"
echo "  ./target/release/polkacomputelab-node --dev"
echo ""
echo "To run tests:"
echo "  cargo test --all"
echo ""
