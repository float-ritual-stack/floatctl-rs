# Floatctl Runtime Scripts

This directory contains the canonical versions of shell scripts used by floatctl's sync daemon.

## Structure

```
scripts/
├── bin/          # Executable scripts (copied to ~/.floatctl/bin/)
│   ├── watch-and-sync.sh      # File watcher daemon for daily notes
│   ├── sync-daily-to-r2.sh    # R2 sync script for daily notes
│   ├── health-check.sh        # System health diagnostics
│   └── cleanup.sh             # Automated cleanup for duplicates/zombies
└── lib/          # Library/helper scripts (copied to ~/.floatctl/lib/)
    ├── log_event.sh           # Structured logging helpers
    └── parse_rclone.sh        # Rclone output parsing
```

## Installation

Install or update scripts to `~/.floatctl/`:

```bash
floatctl sync install
```

This copies all scripts from this directory to your local `~/.floatctl/` installation.

## Development Workflow

When modifying scripts:

1. Edit the canonical version in `scripts/bin/` or `scripts/lib/`
2. Test locally: `floatctl sync install` to copy to `~/.floatctl/`
3. Commit changes to this directory
4. Users upgrade with `cargo install --path floatctl-cli && floatctl sync install`

## Health & Diagnostics

### health-check.sh
System health diagnostics for floatctl/evna infrastructure.

**Checks:**
- Disk space usage
- Memory availability
- Zombie (defunct) processes
- watch-and-sync daemon status
- MCP server duplicates
- evna remote sessions
- Node.js process load
- Docker status

**Usage:**
```bash
~/.floatctl/bin/health-check.sh

# Exit code indicates issue count:
# 0 = all healthy
# >0 = number of issues found
```

### cleanup.sh
Automated cleanup for duplicate processes and stale sessions.

**Cleans:**
- Duplicate MCP servers (keeps 2 oldest)
- Stale evna remote sessions (keeps newest)
- Excessive wrapper processes
- Reports on zombie processes (cannot be killed directly)

**Usage:**
```bash
# Interactive cleanup (asks for confirmation)
~/.floatctl/bin/cleanup.sh

# Preview mode (no changes)
~/.floatctl/bin/cleanup.sh --dry-run

# Automatic cleanup (no prompts)
~/.floatctl/bin/cleanup.sh --force
```

**Example:**
```bash
# Check health first
~/.floatctl/bin/health-check.sh

# Preview cleanup actions
~/.floatctl/bin/cleanup.sh --dry-run

# Run cleanup if issues found
~/.floatctl/bin/cleanup.sh --force
```

## Version Tracking

- Scripts are versioned with the floatctl release
- No separate version numbers for individual scripts
- Breaking changes should be documented in CHANGELOG.md
