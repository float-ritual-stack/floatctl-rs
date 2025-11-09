#!/bin/bash
# Setup user-level system prompt for EVNA
# This allows evna to update her own system prompt without affecting the git repository

set -e

EVNA_DIR="$HOME/.evna"
SOURCE_PROMPT="evna-system-prompt.md"
TARGET_PROMPT="$EVNA_DIR/system-prompt.md"

echo "🔧 Setting up EVNA user-level system prompt..."

# Create ~/.evna directory
if [ ! -d "$EVNA_DIR" ]; then
  echo "📁 Creating $EVNA_DIR"
  mkdir -p "$EVNA_DIR"
fi

# Check if target already exists
if [ -f "$TARGET_PROMPT" ]; then
  echo "⚠️  $TARGET_PROMPT already exists"
  read -p "Overwrite with current template? [y/N] " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "✅ Keeping existing system prompt"
    exit 0
  fi
fi

# Copy system prompt
if [ -f "$SOURCE_PROMPT" ]; then
  echo "📋 Copying $SOURCE_PROMPT to $TARGET_PROMPT"
  cp "$SOURCE_PROMPT" "$TARGET_PROMPT"
  echo "✅ System prompt copied successfully"
else
  echo "❌ Error: $SOURCE_PROMPT not found"
  echo "Run this script from the evna directory"
  exit 1
fi

echo ""
echo "🎉 Setup complete!"
echo ""
echo "EVNA will now load system prompt from: $TARGET_PROMPT"
echo ""
echo "You can now:"
echo "  - Ask EVNA to update her own system prompt"
echo "  - Manually edit $TARGET_PROMPT"
echo "  - Changes persist across git pulls/updates"
echo ""
echo "Backups will be created in $EVNA_DIR when EVNA updates herself"
