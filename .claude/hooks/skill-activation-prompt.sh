#!/bin/bash
set -e

# Get the directory where this script lives
HOOK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HOOK_DIR"
cat | npx tsx skill-activation-prompt.ts
