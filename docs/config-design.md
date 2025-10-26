# Configuration Design Proposal

## Current State

**Environment variables only** (via `.env` files):
- `DATABASE_URL` - PostgreSQL connection string
- `OPENAI_API_KEY` - OpenAI API key for embeddings
- `RUST_LOG` - Logging level

**Command-line arguments** (hardcoded defaults):
- Batch sizes, rate limits, similarity thresholds
- Output formats, paths
- Progress bar settings

## Proposed: TOML Configuration File

Add `~/.floatctl/config.toml` for persistent user preferences.

### Configuration Priority (highest to lowest)
1. Command-line arguments (explicit overrides)
2. Environment variables (`.env` files)
3. `./floatctl.toml` (project-specific config)
4. `~/.floatctl/config.toml` (user defaults)
5. Built-in defaults (hardcoded in Rust)

### Example `~/.floatctl/config.toml`

```toml
# floatctl Configuration
# Location: ~/.floatctl/config.toml

[general]
# Default output directory for extracted conversations
default_output_dir = "~/conversations"

# Default formats to generate (md, json, ndjson)
default_formats = ["md", "ndjson"]

# Show progress bars by default
show_progress = true

# Keep intermediate NDJSON files after full-extract
keep_ndjson = false

[embedding]
# OpenAI model for embeddings
# Options: text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002
model = "text-embedding-3-small"

# Batch size for embedding API calls (1-100)
# Higher = faster but more memory. Recommended: 32-50
batch_size = 32

# Rate limit between API calls (milliseconds)
# Prevents rate limiting. Recommended: 500ms
rate_limit_ms = 500

# Automatically skip already-embedded messages
skip_existing = true

# Token limits for chunking
chunk_size = 6000        # Conservative: 2K buffer below 8192 limit
chunk_overlap = 200      # Overlap for semantic continuity

[query]
# Default result limit
default_limit = 10

# Default similarity threshold (0.0-1.0)
# Lower = stricter (only very similar results)
# Higher = looser (more results)
# null = no filtering
threshold = 0.5

# Default lookback window (days)
default_days = 7

# Output format preference
# Options: "text" (formatted), "json"
output_format = "text"

# Truncate long messages in output
truncate_messages = true
max_message_length = 400

[database]
# Connection pool settings
max_connections = 10
min_connections = 2
connection_timeout_seconds = 30

# Query timeout (seconds)
query_timeout_seconds = 60

[projects]
# Project aliases for fuzzy matching
# Format: canonical_name = ["alias1", "alias2", ...]

[projects.aliases]
"rangle/pharmacy" = ["pharmacy", "pharm", "rx"]
"personal/notes" = ["notes", "personal", "journal"]
"work/meetings" = ["meetings", "work", "standups"]

[artifacts]
# Enable artifact extraction
enabled = true

# Custom artifact type mappings
# Format: "type" = "extension"
[artifacts.extensions]
"application/vnd.ant.react" = "jsx"
"application/vnd.ant.code" = "txt"
"text/html" = "html"
"image/svg+xml" = "svg"
"text/markdown" = "md"

[output]
# Markdown rendering options
include_timestamps = true
include_conversation_id = false

# YAML frontmatter fields
frontmatter_fields = ["title", "created_at", "message_count", "markers"]

# Role indicators (emoji or text)
role_style = "emoji"  # Options: "emoji", "text", "none"

[logging]
# Default log level (overridden by RUST_LOG)
# Options: error, warn, info, debug, trace
default_level = "info"

# Log format
# Options: "compact", "pretty", "json"
format = "compact"

# Enable timestamps in logs
timestamps = true
```

## Configuration Options by Category

### 1. **General Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_output_dir` | Path | `"./conv_out"` | Default extraction output directory |
| `default_formats` | Array | `["md", "json", "ndjson"]` | Default output formats |
| `show_progress` | Bool | `true` | Show progress bars |
| `keep_ndjson` | Bool | `false` | Keep intermediate NDJSON after full-extract |

### 2. **Embedding Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `model` | String | `"text-embedding-3-small"` | OpenAI embedding model |
| `batch_size` | Integer | `32` | API batch size (1-100) |
| `rate_limit_ms` | Integer | `500` | Delay between API calls (ms) |
| `skip_existing` | Bool | `false` | Skip already-embedded messages |
| `chunk_size` | Integer | `6000` | Max tokens per chunk |
| `chunk_overlap` | Integer | `200` | Token overlap between chunks |

### 3. **Query Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_limit` | Integer | `10` | Default result count |
| `threshold` | Float? | `null` | Similarity threshold (0.0-1.0) |
| `default_days` | Integer? | `null` | Default lookback window (days) |
| `output_format` | String | `"text"` | Output format ("text" or "json") |
| `truncate_messages` | Bool | `true` | Truncate long messages |
| `max_message_length` | Integer | `400` | Max message length (chars) |

### 4. **Database Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_connections` | Integer | `10` | Connection pool max size |
| `min_connections` | Integer | `2` | Connection pool min size |
| `connection_timeout_seconds` | Integer | `30` | Connection timeout |
| `query_timeout_seconds` | Integer | `60` | Query timeout |

### 5. **Project Aliases**
| Option | Type | Description |
|--------|------|-------------|
| `projects.aliases` | Table | Project name aliases for fuzzy matching |

Example:
```toml
[projects.aliases]
"rangle/pharmacy" = ["pharmacy", "pharm", "rx"]
"personal/notes" = ["notes", "journal"]
```

### 6. **Artifact Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | Bool | `true` | Enable artifact extraction |
| `extensions` | Table | (see code) | Custom type→extension mappings |

### 7. **Output Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `include_timestamps` | Bool | `true` | Include timestamps in markdown |
| `include_conversation_id` | Bool | `false` | Include conv_id in frontmatter |
| `frontmatter_fields` | Array | `["title", ...]` | YAML frontmatter fields |
| `role_style` | String | `"emoji"` | Role indicator style |

### 8. **Logging Settings**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `default_level` | String | `"info"` | Default log level |
| `format` | String | `"compact"` | Log format |
| `timestamps` | Bool | `true` | Include timestamps |

## Implementation Plan

### Phase 1: Add TOML Support (2-3 hours)
1. Add dependencies:
   ```toml
   [workspace.dependencies]
   config = "0.14"  # Or toml = "0.8"
   serde = { version = "1.0", features = ["derive"] }
   ```

2. Create `floatctl-core/src/config.rs`:
   ```rust
   use serde::{Deserialize, Serialize};
   use std::path::PathBuf;

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FloatctlConfig {
       #[serde(default)]
       pub general: GeneralConfig,
       #[serde(default)]
       pub embedding: EmbeddingConfig,
       #[serde(default)]
       pub query: QueryConfig,
       // ...
   }

   impl FloatctlConfig {
       pub fn load() -> Result<Self> {
           // Priority order:
           // 1. ./floatctl.toml
           // 2. ~/.floatctl/config.toml
           // 3. Built-in defaults
       }
   }
   ```

3. Update commands to use config:
   ```rust
   pub async fn run_embed(args: EmbedArgs) -> Result<()> {
       let config = FloatctlConfig::load()?;

       // CLI args override config
       let batch_size = args.batch_size
           .unwrap_or(config.embedding.batch_size);

       // ...
   }
   ```

### Phase 2: Migration Path (1 hour)
1. Auto-generate `~/.floatctl/config.toml` on first run
2. Add `floatctl config init` command
3. Add `floatctl config show` command (display current config)
4. Add `floatctl config validate` command

### Phase 3: Documentation (1 hour)
1. Update INSTALL.md with config file examples
2. Add docs/configuration.md with full reference
3. Update CLAUDE.md with configuration system

## Benefits

### 1. **User Experience**
- Set preferences once, use everywhere
- No need to remember long command-line arguments
- Project-specific overrides (./floatctl.toml)

### 2. **Power Users**
- Fine-grained control over behavior
- Custom project aliases
- Per-project configuration

### 3. **Defaults Without Magic**
- Explicit configuration over implicit behavior
- Easy to inspect: `floatctl config show`
- Easy to share: commit ./floatctl.toml to git

### 4. **Backward Compatible**
- CLI args still work (highest priority)
- Env vars still work (second priority)
- Config files are optional (fallback to defaults)

## Example Use Cases

### Use Case 1: Power User with Multiple Projects
```bash
# Global config
~/.floatctl/config.toml:
  default_output_dir = "~/conversations"
  batch_size = 50
  show_progress = true

# Project-specific override
~/work/project-a/floatctl.toml:
  [projects.aliases]
  "project-a" = ["proj-a", "pa"]

  [query]
  default_limit = 20
  threshold = 0.6

# Per-command override
floatctl query "error" --limit 5  # Uses limit=5
```

### Use Case 2: Team Collaboration
```bash
# Commit to git
./floatctl.toml:
  [projects.aliases]
  "team/backend" = ["backend", "api", "server"]
  "team/frontend" = ["frontend", "ui", "client"]

  [embedding]
  batch_size = 32

  [artifacts]
  enabled = true

# Team members get consistent behavior
git clone repo
floatctl full-extract --in export.json  # Uses team config
```

### Use Case 3: CI/CD Pipeline
```bash
# CI config
./floatctl.toml:
  [general]
  show_progress = false
  keep_ndjson = true

  [logging]
  format = "json"

  [embedding]
  batch_size = 10  # Lower for CI stability
  rate_limit_ms = 1000

# CI script
floatctl embed --in messages.ndjson  # Uses CI-friendly settings
```

## Alternative: Stick with CLI Args + Env Vars?

**Pros of current approach:**
- Simple, no new dependencies
- UNIX philosophy: everything via CLI
- Easy to script

**Cons of current approach:**
- Have to remember/type long arguments
- No way to set persistent preferences
- No project-specific settings
- Hard to share team conventions

**Recommendation:** Implement TOML config for Phase 2+ users, keep CLI-only workflow for quick scripts.

## Questions for Discussion

1. **Which options are most valuable?** Start with core subset?
2. **Use `config` crate or manual TOML parsing?** `config` supports multiple formats (TOML, YAML, JSON)
3. **Support environment variable overrides?** e.g., `FLOATCTL_BATCH_SIZE=50`
4. **Generate default config on first run?** Or require manual `floatctl config init`?
5. **Validate config on load?** Strict validation vs. permissive with warnings?

## Next Steps

1. Get feedback on proposed options
2. Implement Phase 1 (basic TOML support)
3. Dogfood with real usage
4. Iterate based on pain points
