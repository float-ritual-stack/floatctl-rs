#!/bin/bash
# Quick Deploy - floatctl to local + float-box
# Usage: ./scripts/bin/quick-deploy.sh [--local-only] [--remote-only]
#
# Does all the things we keep doing manually:
# 1. Install locally (macOS) with all features
# 2. Push to git
# 3. Build on float-box with server feature
# 4. Deploy to docker container + the-magic bootstrap

set -e
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

LOCAL_ONLY=""
REMOTE_ONLY=""

for arg in "$@"; do
  case $arg in
    --local-only) LOCAL_ONLY=1 ;;
    --remote-only) REMOTE_ONLY=1 ;;
    --help|-h)
      echo "Usage: $0 [--local-only] [--remote-only]"
      echo ""
      echo "  --local-only   Only install locally (skip float-box)"
      echo "  --remote-only  Only deploy to float-box (skip local)"
      exit 0
      ;;
  esac
done

cd "$REPO_ROOT"

echo "▒▒ FLOATCTL QUICK DEPLOY ▒▒"
echo ""

# ─────────────────────────────────────────────────────────────
# LOCAL BUILD (macOS)
# ─────────────────────────────────────────────────────────────
if [[ -z "$REMOTE_ONLY" ]]; then
  echo "→ Building locally (macOS)..."
  cargo build --release -p floatctl-cli --features embed,server 2>&1 | tail -5

  echo "→ Installing to ~/.cargo/bin/floatctl..."
  cp target/release/floatctl ~/.cargo/bin/floatctl

  echo "  ✓ Local: $(floatctl --version 2>/dev/null | head -1)"
  echo ""
fi

# ─────────────────────────────────────────────────────────────
# GIT PUSH (needed for float-box to pull)
# ─────────────────────────────────────────────────────────────
if [[ -z "$LOCAL_ONLY" ]]; then
  # Check if there are uncommitted OR staged changes
  if ! git diff --quiet HEAD 2>/dev/null || ! git diff --quiet --cached 2>/dev/null; then
    echo "⚠ Uncommitted or staged changes detected. Commit first or they won't deploy to float-box."
    echo "  (Continuing with last committed version...)"
    echo ""
  fi

  # Push if we have commits ahead of remote (using git rev-list for locale independence)
  if [[ $(git rev-list @{u}..HEAD 2>/dev/null | wc -l) -gt 0 ]]; then
    echo "→ Pushing to git..."
    git push
    echo ""
  fi

  # ─────────────────────────────────────────────────────────────
  # FLOAT-BOX BUILD + DEPLOY
  # ─────────────────────────────────────────────────────────────
  echo "→ Building on float-box (with --features server)..."
  ssh float-box "cd ~/float-hub-operations/floatctl-rs && git pull -q && source ~/.cargo/env && cargo build --release -p floatctl-cli --features server 2>&1" | tail -5

  echo "→ Deploying to float-box..."

  # Stop container, copy binary, start container
  ssh float-box "cd /opt/float && \
    docker compose stop floatctl-serve 2>/dev/null && \
    cp ~/float-hub-operations/floatctl-rs/target/release/floatctl /opt/float/bin/ && \
    docker compose start floatctl-serve 2>/dev/null"

  # Update the-magic bootstrap binary
  ssh float-box "cp ~/float-hub-operations/floatctl-rs/target/release/floatctl /opt/float/bbs/the-magic/floatctl-linux-x86_64"

  # Verify using health endpoint (more reliable than log parsing)
  sleep 2
  if ssh float-box "curl -sf http://localhost:3030/health &>/dev/null"; then
    echo "  ✓ Server: responding on :3030"
  else
    echo "  ⚠ Server health check failed. Recent logs:"
    ssh float-box "docker logs floatctl-serve --tail 10" || true
  fi

  REMOTE_VERSION=$(ssh float-box "/opt/float/bin/floatctl --version 2>/dev/null | head -1" || echo "unknown")
  echo "  ✓ Float-box: $REMOTE_VERSION"
  echo "  ✓ Bootstrap: /opt/float/bbs/the-magic/floatctl-linux-x86_64"
  echo ""
fi

echo "▒▒ DEPLOY COMPLETE ▒▒"
echo ""
echo "Endpoints:"
echo "  Local:  floatctl --version"
echo "  Server: https://float-bbs.ngrok.io/health"
echo "  Magic:  curl -sL https://float-bbs.ngrok.io/the-magic/bootstrap.sh | bash"
