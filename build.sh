#!/usr/bin/env bash
# Build script for OpenAirLLM By EdgeSearch
# Compiles the Rust workspace and builds the Node.js frontend.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR" && pwd)"

echo "🔨 Building Rust workspace..."
cd "$PROJECT_ROOT"
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings

echo "📦 Building frontend..."
cd "$PROJECT_ROOT/frontend"
bun install
bun run build

echo "✅ Build complete!"
echo ""
echo "Binaries:"
echo "  Bridge:  $PROJECT_ROOT/target/release/bridge"
echo "  CLI:     $PROJECT_ROOT/target/release/airllm"
echo "  Frontend: $PROJECT_ROOT/frontend/bin/openairllm"
echo ""
echo "To start: ./frontend/bin/openairllm-launch"