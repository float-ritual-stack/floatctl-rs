# Installing floatctl Globally

This guide explains how to install `floatctl` as a global command-line tool and configure it with a global `.env` file.

## Quick Start

```bash
# 1. Install globally
cargo install --path floatctl-cli --features embed

# 2. Create global config directory
mkdir -p ~/.floatctl

# 3. Copy your environment variables
cp .env ~/.floatctl/.env

# 4. Run from anywhere
cd /any/directory
floatctl query "search term"
```

## Installation Methods

### Method 1: Install from Source (Recommended)

```bash
# From the floatctl-rs directory
cargo install --path floatctl-cli --features embed

# This installs to ~/.cargo/bin/floatctl
# Ensure ~/.cargo/bin is in your PATH
```

### Method 2: Install with All Features

```bash
# Install with all optional features
cargo install --path floatctl-cli --features embed --locked

# Or build optimized release
cargo build --release --features embed
sudo cp target/release/floatctl /usr/local/bin/
```

## Configuration: Global `.env` File

### Setting Up ~/.floatctl/.env

The `floatctl` tool now supports a global configuration file at `~/.floatctl/.env`. This allows you to:
- Install once, use from anywhere
- Share configuration across all your projects
- Override with local `.env` files when needed

**Create the config directory:**
```bash
mkdir -p ~/.floatctl
```

**Create ~/.floatctl/.env:**
```bash
cat > ~/.floatctl/.env << 'EOF'
# PostgreSQL/Supabase connection
DATABASE_URL="postgresql://user:password@host:port/floatctl"

# OpenAI API key (for embeddings)
OPENAI_API_KEY="sk-..."

# Optional: Rust logging level
RUST_LOG="info"
EOF
```

**Security reminder:**
```bash
# Protect your secrets
chmod 600 ~/.floatctl/.env
```

## How Configuration Loading Works

`floatctl` checks for `.env` files in **priority order** (highest to lowest):

1. **Current directory `.env`** - Highest priority
   - Useful for project-specific overrides
   - Example: Local dev database for testing

2. **`~/.floatctl/.env`** - Global defaults
   - Used when no local `.env` exists
   - Your main production configuration

3. **Environment variables already set** - Lowest priority
   - From your shell (`.bashrc`, `.zshrc`, etc.)
   - Not overwritten by .env files

**Example:**
```bash
# Global config (production database)
~/.floatctl/.env:
  DATABASE_URL=postgresql://prod-server/floatctl

# Local override (dev database)
./my-project/.env:
  DATABASE_URL=postgresql://localhost/floatctl_dev

# Result when running from ./my-project:
# Uses local dev database (current directory wins)

# Result when running from ~/ or any other directory:
# Uses global production database (from ~/.floatctl/.env)
```

## Usage Examples

### Basic Usage (From Anywhere)

```bash
# Embedding - works from any directory
cd ~/Documents
floatctl embed --in ~/conversations/messages.ndjson

# Query - uses global config
floatctl query "error handling patterns" --limit 5

# Full extraction
floatctl full-extract --in ~/Downloads/export.json --out ~/archive/
```

### Project-Specific Overrides

```bash
# Create a project with local config
mkdir -p ~/projects/my-experiment
cd ~/projects/my-experiment

# Local .env overrides global config
cat > .env << 'EOF'
DATABASE_URL="postgresql://localhost:5433/experiment_db"
OPENAI_API_KEY="sk-test-key-..."
EOF

# Now commands use local config
floatctl query "test data" --project my-experiment
```

### Checking Configuration

The `embed` and `query` commands log which configuration files were loaded:

```bash
$ floatctl query "search term" 2>&1 | grep "Loaded configuration"
Loaded configuration from: current directory (./.env), ~/.floatctl/.env (~/.floatctl/.env)
```

If no `.env` files are found:
```bash
$ floatctl query "search term" 2>&1 | grep "Using environment"
Using environment variables only (no .env file found)
```

## Updating Configuration

### Changing Global Config

```bash
# Edit global config
nano ~/.floatctl/.env

# Or use environment variable for DATABASE_URL
export DATABASE_URL="postgresql://new-host/floatctl"
```

### Switching Between Environments

Create multiple config files:

```bash
# Production
~/.floatctl/.env

# Staging
~/.floatctl/.env.staging

# Development
~/.floatctl/.env.dev
```

Load specific environment:
```bash
# Copy the desired config
cp ~/.floatctl/.env.staging ~/.floatctl/.env

# Or use environment variable
export DATABASE_URL="postgresql://staging-server/floatctl"
floatctl query "search term"
```

## Troubleshooting

### "DATABASE_URL not set" Error

This means no `.env` file was found and the variable isn't set in your shell.

**Solution:**
```bash
# Check if global config exists
ls -la ~/.floatctl/.env

# If missing, create it
mkdir -p ~/.floatctl
echo 'DATABASE_URL="postgresql://..."' > ~/.floatctl/.env

# Or set in current shell
export DATABASE_URL="postgresql://user:pass@host/db"
```

### "Command not found: floatctl"

The installation directory isn't in your PATH.

**Solution:**
```bash
# Check installation location
which floatctl

# If not found, ensure ~/.cargo/bin is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Or for zsh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Config Not Loading

Enable debug logging to see which files are checked:

```bash
RUST_LOG=debug floatctl query "search term" 2>&1 | grep -i "env\|config"
```

Output shows:
- Which directories were checked
- Which files were found and loaded
- The priority order

## Uninstallation

```bash
# Remove the binary
cargo uninstall floatctl

# Or if installed manually
rm /usr/local/bin/floatctl

# Remove global config (optional)
rm -rf ~/.floatctl
```

## Shell Completions

`floatctl` supports shell completions for bash, zsh, fish, PowerShell, and Elvish. Tab completion works for all commands, subcommands, and options.

### Generate Completion Scripts

```bash
# Bash
floatctl completions bash > ~/.local/share/bash-completion/completions/floatctl

# Zsh
floatctl completions zsh > ~/.zsh/completions/_floatctl

# Fish
floatctl completions fish > ~/.config/fish/completions/floatctl.fish

# PowerShell
floatctl completions powershell > ~/floatctl.ps1

# Elvish
floatctl completions elvish > ~/floatctl.elv
```

### Installation by Shell

#### Bash

**Option 1: User-specific (recommended)**
```bash
# Create completions directory
mkdir -p ~/.local/share/bash-completion/completions

# Generate completion script
floatctl completions bash > ~/.local/share/bash-completion/completions/floatctl

# Reload bash (or restart terminal)
exec bash
```

**Option 2: System-wide**
```bash
# Generate completion script (requires sudo)
sudo floatctl completions bash > /usr/share/bash-completion/completions/floatctl

# Reload bash
exec bash
```

#### Zsh

**Option 1: User-specific (recommended)**
```bash
# Create completions directory
mkdir -p ~/.zsh/completions

# Generate completion script
floatctl completions zsh > ~/.zsh/completions/_floatctl

# Add to ~/.zshrc (if not already present)
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload zsh
exec zsh
```

**Option 2: Oh My Zsh**
```bash
# Generate completion script
floatctl completions zsh > ~/.oh-my-zsh/custom/plugins/floatctl/_floatctl

# Reload zsh
exec zsh
```

#### Fish

```bash
# Create completions directory (if needed)
mkdir -p ~/.config/fish/completions

# Generate completion script
floatctl completions fish > ~/.config/fish/completions/floatctl.fish

# Fish will automatically load completions (no restart needed)
```

#### PowerShell

```powershell
# Generate completion script
floatctl completions powershell | Out-File -FilePath $HOME\Documents\PowerShell\Scripts\floatctl.ps1

# Add to your PowerShell profile
Add-Content $PROFILE ". $HOME\Documents\PowerShell\Scripts\floatctl.ps1"

# Reload profile
. $PROFILE
```

### Testing Completions

After installation, test that completions work:

```bash
# Type the command and press TAB
floatctl <TAB>

# Should show:
# bridge       completions  embed        embed-notes  evna
# explode      full-extract help         ndjson       query
# split        sync

# Try subcommands
floatctl bridge <TAB>

# Should show:
# index  help
```

### Troubleshooting Completions

**Bash: Completions not working**
```bash
# Check if bash-completion is installed
which bash-completion

# Ubuntu/Debian
sudo apt install bash-completion

# macOS (Homebrew)
brew install bash-completion@2
```

**Zsh: Completions not loading**
```bash
# Rebuild completion cache
rm ~/.zcompdump*
autoload -Uz compinit && compinit

# Check fpath includes your completions directory
echo $fpath | grep -o ~/.zsh/completions
```

**Fish: Completions not appearing**
```bash
# Check fish can find the completion file
ls ~/.config/fish/completions/floatctl.fish

# Force reload
fish_update_completions
```

## Advanced: Shell Integration

### Bash/Zsh Alias

Add convenience aliases to your shell config:

```bash
# ~/.bashrc or ~/.zshrc

# Quick query
alias fq='floatctl query'

# Quick embed
alias fe='floatctl embed'

# Use production config
alias floatctl-prod='DATABASE_URL=postgresql://prod-host/floatctl floatctl'

# Use staging config
alias floatctl-staging='DATABASE_URL=postgresql://staging-host/floatctl floatctl'
```

### Environment Switching Function

```bash
# ~/.bashrc or ~/.zshrc

floatctl-env() {
  local env="${1:-prod}"
  local config="$HOME/.floatctl/.env.$env"

  if [ -f "$config" ]; then
    cp "$config" "$HOME/.floatctl/.env"
    echo "✓ Switched to $env environment"
  else
    echo "✗ Config not found: $config"
    return 1
  fi
}

# Usage:
# floatctl-env prod
# floatctl-env staging
# floatctl-env dev
```

## Security Best Practices

1. **Protect your config files:**
   ```bash
   chmod 600 ~/.floatctl/.env
   chmod 700 ~/.floatctl
   ```

2. **Never commit secrets to git:**
   ```bash
   # Add to .gitignore
   echo "~/.floatctl/" >> ~/.gitignore
   ```

3. **Use different credentials for dev/prod:**
   - Global config: production database (read-only)
   - Local config: development database (full access)

4. **Rotate API keys regularly:**
   - OpenAI API keys should be rotated every 90 days
   - Use key restrictions when available

## Migration from Local Install

If you were running `cargo run` before:

```bash
# Old way (from floatctl-rs directory)
cd ~/floatctl-rs
cargo run --release -p floatctl-cli --features embed -- query "search"

# New way (from anywhere)
cd ~/anywhere
floatctl query "search"
```

## Next Steps

- Read [CLAUDE.md](./CLAUDE.md) for project architecture
- See [floatctl-embed/README.md](./floatctl-embed/README.md) for embedding details
- Check [examples/](./examples/) for usage examples

## Feedback

Found an issue? Please report at:
- GitHub Issues: (your repo URL)
- Or submit a PR with improvements
