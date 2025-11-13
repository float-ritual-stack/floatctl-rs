# Floatctl Runtime Scripts

This directory contains the canonical versions of shell scripts used by floatctl's sync daemon.

## Structure

```
scripts/
├── bin/          # Executable scripts (copied to ~/.floatctl/bin/)
│   ├── watch-and-sync.sh      # File watcher daemon for daily notes
│   └── sync-daily-to-r2.sh    # R2 sync script for daily notes
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

## Version Tracking

- Scripts are versioned with the floatctl release
- No separate version numbers for individual scripts
- Breaking changes should be documented in CHANGELOG.md
