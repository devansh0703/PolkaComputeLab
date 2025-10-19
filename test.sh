#!/bin/bash

set -e

echo "🧪 Running PolkaComputeLab Tests..."
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust."
    exit 1
fi

echo "📦 Testing Job Registry Pallet..."
cargo test -p pallet-job-registry --lib -- --nocapture

echo ""
echo "📦 Testing Job Verifier Pallet..."
cargo test -p pallet-job-verifier --lib -- --nocapture

echo ""
echo "📦 Testing Consensus Manager Pallet..."
cargo test -p pallet-consensus-manager --lib -- --nocapture

echo ""
echo "📦 Testing Event Hub Pallet..."
cargo test -p pallet-event-hub --lib -- --nocapture

echo ""
echo "📦 Testing Telemetry Pallet..."
cargo test -p pallet-telemetry --lib -- --nocapture

echo ""
echo "📦 Testing Runtime..."
cargo test -p polkacomputelab-runtime

echo ""
echo "✅ All tests passed!"
echo ""
