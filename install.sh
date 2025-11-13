#!/usr/bin/env bash

set -euo pipefail

# Validate cargo is available
if ! command -v cargo &> /dev/null; then
  echo "Error: cargo not found. Please install Rust." >&2
  exit 1
fi

echo "Installing floatctl-cli with embed feature..."

if cargo install --path floatctl-cli --features embed; then
  echo "✓ floatctl-cli installed successfully"
else
  echo "✗ Installation failed" >&2
  exit 1
fi
