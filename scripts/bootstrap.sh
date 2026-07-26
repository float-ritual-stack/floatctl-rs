#!/bin/bash
# Float Workshop Bootstrap - Pocket Check Tools
# Usage: curl -sL https://bbs.floatbbs.net/the-magic/bootstrap.sh | bash
#
# Multi-arch: detects uname -m and downloads matching binaries.
# Supported: x86_64 (Desktop Claude sandbox), aarch64 (cowork/Code sessions)

set -e

MAGIC_URL="${MAGIC_URL:-https://bbs.floatbbs.net/the-magic}"
INSTALL_DIR="${INSTALL_DIR:-/tmp}"
BBS_ENDPOINT="https://bbs.floatbbs.net"

# ─── Download verification ───────────────────────────────────
# A dead host, SPA catch-all, or ngrok 404 returns HTTP 200 with an HTML error
# page, so `curl -o` succeeds and writes HTML where a binary should be — the
# failure then surfaces two steps later as "Syntax error: newline unexpected"
# and reads like a corrupt download. Both floatctl and jq are ELF binaries, so
# reject anything whose magic bytes aren't ELF and name the URL that lied.
verify_binary() {
  local path="$1" url="$2"
  if [ ! -s "$path" ]; then
    echo "✗ Download failed (empty file): $url"
    rm -f "$path"
    exit 1
  fi
  if [ "$(head -c 4 "$path" | od -An -tx1 | tr -d ' \n')" != "7f454c46" ]; then
    echo "✗ Not a binary: $url"
    echo "  Downloaded a non-ELF file — likely an HTML error page (wrong host or path):"
    head -c 160 "$path" | tr -d '\0' | sed 's/^/    /'
    echo ""
    rm -f "$path"
    exit 1
  fi
}

# ─── Architecture detection ──────────────────────────────────
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)
    FLOATCTL_ASSET="floatctl-linux-x86_64"
    JQ_ASSET="jq-linux-amd64"
    ;;
  aarch64|arm64)
    FLOATCTL_ASSET="floatctl-linux-aarch64"
    JQ_ASSET="jq-linux-arm64"
    ;;
  *)
    echo "✗ Unsupported architecture: $ARCH"
    echo "  Supported: x86_64, aarch64"
    exit 1
    ;;
esac

echo "▒▒ FLOAT WORKSHOP BOOTSTRAP ▒▒"
echo "  arch: $ARCH → $FLOATCTL_ASSET"
echo ""

# floatctl - BBS operations, search, etc
echo "→ Acquiring floatctl ($FLOATCTL_ASSET)..."
curl -sL -o "$INSTALL_DIR/floatctl" "$MAGIC_URL/$FLOATCTL_ASSET"
verify_binary "$INSTALL_DIR/floatctl" "$MAGIC_URL/$FLOATCTL_ASSET"
chmod +x "$INSTALL_DIR/floatctl"

# jq - JSON filtering (context window defense)
echo "→ Acquiring jq ($JQ_ASSET)..."
curl -sL -o "$INSTALL_DIR/jq" "https://github.com/jqlang/jq/releases/download/jq-1.7.1/$JQ_ASSET"
verify_binary "$INSTALL_DIR/jq" "https://github.com/jqlang/jq/releases/download/jq-1.7.1/$JQ_ASSET"
chmod +x "$INSTALL_DIR/jq"

echo ""
echo "▒▒ TOOLS ACQUIRED ▒▒"
echo ""
echo "  floatctl: $INSTALL_DIR/floatctl"
echo "  jq:       $INSTALL_DIR/jq"
echo ""

# Quick verification
if "$INSTALL_DIR/floatctl" --version >/dev/null 2>&1; then
  echo "  ✓ floatctl $("$INSTALL_DIR/floatctl" --version 2>/dev/null | head -1)"
else
  echo "  ✗ floatctl verification failed"
fi

if "$INSTALL_DIR/jq" --version >/dev/null 2>&1; then
  echo "  ✓ $("$INSTALL_DIR/jq" --version)"
else
  echo "  ✗ jq verification failed"
fi

echo ""
echo "▒▒ EPHEMERAL CONTEXT SETUP ▒▒"
echo ""
echo "  export FLOATCTL_BBS_ENDPOINT=\"$BBS_ENDPOINT\""
echo ""
echo "  ⚠️  RUN THIS EXPORT before using floatctl bbs commands!"
echo "  (Ephemeral sandboxes cant reach float-box directly)"
echo ""

# Show what is on deck - why you are here
echo "▒▒ ON DECK (current priorities) ▒▒"
echo ""
LATEST=$(ls -t /opt/float/bbs/boards/on-deck/*.md 2>/dev/null | head -1)
if [ -n "$LATEST" ] && [ -f "$LATEST" ]; then
  # Extract title from frontmatter
  TITLE=$(grep "^title:" "$LATEST" 2>/dev/null | head -1 | sed "s/^title: *//")
  AUTHOR=$(grep "^author:" "$LATEST" 2>/dev/null | head -1 | sed "s/^author: *//")
  echo "  📋 $TITLE (by $AUTHOR)"
  echo ""
  # Show content after frontmatter, first 15 lines
  sed -n "/^---$/,/^---$/d; p" "$LATEST" | head -15 | sed "s/^/  /"
  echo ""
  echo "  ..."
  echo ""
fi

echo "∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿"
echo "The magic is ready."
echo ""
echo "Quick start:"
echo "  export FLOATCTL_BBS_ENDPOINT=\"$BBS_ENDPOINT\""
echo "  $INSTALL_DIR/floatctl bbs board list on-deck --persona daddy --insecure"
echo "  $INSTALL_DIR/floatctl bbs inbox --persona daddy --insecure"
