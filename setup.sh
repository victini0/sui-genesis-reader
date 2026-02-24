#!/bin/bash
# Automated setup script for sui-genesis-reader
# This script clones the required repositories and builds the analyzer.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== sui-genesis-reader setup ==="
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust toolchain not found."
    echo "Install from: https://rustup.rs/"
    exit 1
fi

# Clone Sui repo if needed
if [ ! -d "$SCRIPT_DIR/../sui" ]; then
    echo "[1/4] Cloning Sui repository (this may take a while)..."
    git clone --depth 1 https://github.com/MystenLabs/sui.git "$SCRIPT_DIR/../sui"
else
    echo "[1/4] Sui repository already exists, skipping clone."
fi

# Clone genesis blob if needed
if [ ! -d "$SCRIPT_DIR/../sui-genesis" ]; then
    echo "[2/4] Cloning genesis blob..."
    git clone https://github.com/MystenLabs/sui-genesis.git "$SCRIPT_DIR/../sui-genesis"
else
    echo "[2/4] Genesis blob already exists, skipping clone."
fi

# Copy analyzer into Sui's crates
echo "[3/4] Copying analyzer into Sui crates directory..."
cp -r "$SCRIPT_DIR" "$SCRIPT_DIR/../sui/crates/sui-genesis-reader"

# Build and run
echo "[4/4] Building and running (first build takes ~10 minutes)..."
echo ""
cd "$SCRIPT_DIR/../sui"
cargo run --release -p sui-genesis-reader -- "$SCRIPT_DIR/../sui-genesis/mainnet/genesis.blob" | tee "$SCRIPT_DIR/output/genesis-analysis.txt"

echo ""
echo "=== Done! Output saved to output/genesis-analysis.txt ==="
