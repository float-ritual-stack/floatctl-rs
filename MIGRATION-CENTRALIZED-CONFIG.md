# Migration Guide: Centralized Configuration

This guide covers migrating from distributed configs to the centralized `~/.floatctl/config.toml` system.

## What Changed

**Before**: N tools × M configs = maintenance hell
- `workspace-context.json` with hardcoded `/Users/evan` paths
- Multiple `.env` files with path configuration
- `.claude/settings.local.json` with absolute paths
- Config drift between machines

**After**: 1 config × N tools = single source of truth
- `~/.floatctl/config.toml` - centralized paths and settings
- Machine-specific overrides for Mac vs Hetzner
- Runtime variable expansion (`${float_home}`, `${HOME}`)
- Config validation with `floatctl config validate`

## Migration Path

### Step 1: Initialize Centralized Config

```bash
# Auto-detect current paths and create config
floatctl config init --detect

# Or manually copy template
cp floatctl-rs/.floatctl-config.template.toml ~/.floatctl/config.toml
```

**What this does**:
- Creates `~/.floatctl/config.toml` with auto-detected paths
- Replaces `/Users/evan` with your actual home directory
- Validates that `float-hub` exists at expected location

### Step 2: Validate Configuration

```bash
# Check that all paths exist and config is valid
floatctl config validate
```

**Expected output**:
```
🔍 Validating configuration...
   ✓ Config loaded successfully
   Machine: macbook-pro (local)
   ✓ All paths exist and are accessible
   ⚠  evna.database_url not set or using unresolved env var

✅ Configuration valid!
```

### Step 3: Update Machine-Specific Overrides (Hetzner Example)

Edit `~/.floatctl/config.toml` and add machine overrides:

```toml
[paths."hetzner-box"]
float_home = "/mnt/float-01/evan-brain"
daily_notes_home = "/mnt/float-01/evan-brain/.evans-notes"
# Other paths auto-derived from ${float_home}

[evna."hetzner-box"]
database_url = "postgresql://localhost:5433/floatctl"
mcp_server_port = 3001  # Different port on server
```

**On Hetzner**, set machine name:
```bash
export FLOATCTL_MACHINE=hetzner-box
floatctl config validate  # Verifies Hetzner paths
```

### Step 4: Tools Updated (Already Done)

The following tools now read from centralized config:

✅ **evna** (`evna/src/tools/index.ts`)
- `BrainBootTool` → uses `config.paths.daily_notes`
- `BridgeHealthTool` → uses `config.paths.bridges`
- Graceful fallback if config missing

✅ **floatctl** (all commands)
- Reads config for conversation exports, scripts, cache directories

⏳ **TODO** (not yet migrated):
- `r2-sync` scripts → update to use `config.paths.*`
- `floatctl-bridge` → update to use `config.paths.bridges`
- Shell scripts in `~/.floatctl/scripts/` → source `floatctl config export`

### Step 5: Update Shell Scripts (Optional)

For bash scripts that need paths:

```bash
#!/bin/bash
# Source floatctl config as environment variables
eval "$(floatctl config export)"

echo "Using FLOAT_HOME: $FLOAT_HOME"
echo "Bridges at: $FLOAT_BRIDGES"

# Access specific values
DAILY_NOTES=$(floatctl config get paths.daily_notes)
```

### Step 6: Cleanup Old Configs (When Safe)

**DO NOT delete until you've verified everything works!**

Old configs that can be removed after migration:
- ~~`evna/src/config/workspace-context.json`~~ (KEEP - still used for project/user metadata)
- ~~`.env` files with path configuration~~ (KEEP - still used for API keys)
- `.claude/settings.local.json` Read() rules with hardcoded paths (update to relative)

**What to keep**:
- `workspace-context.json` - Project aliases, user info, meeting metadata
- `.env` files - API keys, database URLs (referenced via `${VAR}`)
- Claude Code settings - Just update paths to be relative or use variables

## Benefits

### 1. Single Source of Truth

Change `float_home` once → all tools see it:

```toml
[paths]
float_home = "/Users/evan/float-hub"  # Change this
daily_notes = "${float_home}/.evans-notes/daily"  # Auto-updates
bridges = "${float_home}/float.dispatch/bridges"  # Auto-updates
```

### 2. Machine-Specific Overrides

Same config file, different machines:

```bash
# Mac
floatctl config get paths.float_home
# /Users/evan/float-hub

# Hetzner (with FLOATCTL_MACHINE=hetzner-box)
floatctl config get paths.float_home
# /mnt/float-01/evan-brain
```

### 3. Validation

Catch config issues early:

```bash
floatctl config validate
# ✗ paths.bridges: /bad/path (does not exist)
# ✗ paths.daily_notes: /tmp/file.txt (not a directory)
```

### 4. Shell Integration

Export as environment variables:

```bash
source <(floatctl config export)
# Now you have: $FLOAT_HOME, $FLOAT_BRIDGES, etc.
```

## Troubleshooting

### Config Not Found

```
Error: Config not found at /Users/evan/.floatctl/config.toml

Run: floatctl config init --detect
```

**Solution**: Initialize config with `floatctl config init --detect`

### Path Validation Failed

```
❌ Path validation failed:
  ✗ float_home: /Users/evan/float-hub (does not exist)
```

**Solution**: Create the directory or update the path in config:

```bash
# Create directory
mkdir -p /Users/evan/float-hub

# OR update config
vim ~/.floatctl/config.toml
# Change float_home = "/correct/path"
```

### Environment Variable Not Expanding

```toml
database_url = "${DATABASE_URL}"  # Shows as literal string
```

**Cause**: Environment variable not set

**Solution**: Set in your shell or `.env` file:

```bash
export DATABASE_URL="postgresql://..."
```

### Evna Falls Back to Default Paths

```
[config] Failed to load centralized config - falling back to defaults
```

**Cause**: Config file doesn't exist or has syntax error

**Solution**:
1. Run `floatctl config path` to see expected location
2. Run `floatctl config init --detect` to create
3. Check TOML syntax if file exists

## Commands Reference

```bash
# Initialize config
floatctl config init --detect

# Get specific value
floatctl config get paths.float_home

# List all config
floatctl config list

# Validate paths
floatctl config validate

# Export as env vars
floatctl config export
source <(floatctl config export)

# Show config file path
floatctl config path
```

## Config File Structure

```toml
[machine]
name = "macbook-pro"
environment = "local"

[paths]
float_home = "/Users/evan/float-hub"
daily_notes_home = "/Users/evan/.evans-notes"
daily_notes = "${daily_notes_home}/daily"
bridges = "${float_home}/float.dispatch/bridges"
operations = "${float_home}/operations"
inbox = "${float_home}/inbox"
dispatches = "${float_home}/float.dispatch"

[evna]
database_url = "${DATABASE_URL}"
mcp_server_port = 3000

[floatctl]
cache_dir = "${HOME}/.cache/floatctl"
scripts_dir = "${HOME}/.floatctl/scripts"

[r2]
enabled = true
bucket_name = "float-backups"
account_id = "${R2_ACCOUNT_ID}"
api_token = "${R2_API_TOKEN}"

# Machine overrides
[paths."hetzner-box"]
float_home = "/mnt/float-01/evan-brain"
daily_notes_home = "/mnt/float-01/evan-brain/.evans-notes"
```

## Next Steps

1. ✅ Initialize config: `floatctl config init --detect`
2. ✅ Validate: `floatctl config validate`
3. ⏳ Update remaining tools (r2-sync, scripts)
4. ⏳ Test on Hetzner with machine override
5. ⏳ Remove old hardcoded configs (when safe)

## Philosophy

This solves the **N×M maintenance problem**:
- N tools reading from M different configs = N×M update points
- All tools reading from 1 config = N update points (way better)

**Shacks not cathedrals**: Simple TOML file, not a config server. Tools read it directly or via `floatctl config get`. No over-engineering.
