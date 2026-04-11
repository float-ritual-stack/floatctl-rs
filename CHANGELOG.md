# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Multi-arch Linux bootstrap (aarch64 + x86_64)**
  - New `Cross.toml` configures `cross` for reproducible aarch64 cross-compiles from float-box
  - New `scripts/bootstrap.sh` (version-controlled copy of `/opt/float/bbs/the-magic/bootstrap.sh`)
    detects `uname -m` and downloads the matching `floatctl-linux-{x86_64,aarch64}` + jq binary
  - aarch64 binary has GLIBC 2.18 floor + pure-Rust TLS, so it runs in any Linux sandbox from
    ~2013 onward (Desktop Claude x86_64, cowork/Code aarch64, and future targets)
  - **Why**: cowork / Claude Code sessions run in aarch64 Linux sandboxes and failed with
    `Exec format error` on the old single-arch x86_64-only bootstrap

### Changed

- **sqlx TLS backend: native-tls → rustls** (`tls-native-tls` → `tls-rustls` in workspace Cargo.toml)
  - Removes transitive openssl-sys dependency, aligning with `reqwest`'s existing `rustls-tls`
  - Produces statically-linked TLS across both the server feature and embed feature
  - Binary no longer dynamically links `libssl.so` — portable across distros
  - Postgres wire protocol behaviour is unchanged; only the TLS handshake backend differs

### Added

- **Artifact Extraction: create_file + antArtifact support**
  - Extract artifacts from `create_file` tool_use blocks (Claude Desktop sandbox files)
  - Extract artifacts from `<antArtifact>` XML tags in text blocks (older conversations)
  - Deduplication for `create_file` artifacts (last-write-wins when same file is rewritten)
  - Markdown renderer now annotates Desktop artifacts with `📎 File:` markers
  - **Impact**: Test export went from 20 extracted artifacts → 590 across 190 conversations
  - Three artifact formats now covered: web `artifacts` tool, Desktop `create_file`, legacy `<antArtifact>` XML

- **Marker Extraction: general `word::value` pattern**
  - Replaced hardcoded 8-prefix regex with general `word::value` pattern
  - Full `ctx::` line capture including timestamp (`ctx::2026-03-21 @ 10:10:38 PM ...`)
  - Bracketed marker support (`[project::X]`, `[mode::Y]`, `[session::abc]`)
  - Code fence and backtick stripping — markers inside code blocks are ignored
  - Now captures: `mode::`, `session::`, `branch::`, `sc::`, `slug::`, `plan::`, `type::`, `dispatch::`, `bridge::`, `worklog::`, `system_status::`, and any future `word::` markers

## [0.3.1] - 2025-12-26

### Fixed

- **R2 Search Moved to Server-Side API** (PR #52)
  - `floatctl bbs get` now calls server API for R2 bucket search instead of spawning local `rclone`
  - New server endpoints: `GET /bbs/r2/search` and `GET /bbs/r2/files/{*path}`
  - **Why**: Clients in sandboxed environments (Claude Desktop MCP) don't have rclone installed
  - Docker container now mounts rclone binary and CA certificates for TLS verification
  - Eliminates silent 0-result failures in sandbox environments

- **Dynamic Status Injection Removed**
  - Removed dynamic status injection from tool descriptions (caused issues with static tool registration)

## [0.3.0] - 2025-12-19

### Added

- **BBS File Search via Server API**
  - `get_search_paths` option in `[bbs]` config section - list of filesystem paths to search
  - Server endpoint `GET /bbs/files?q=<query>&limit=<n>` for fuzzy file search
  - Server endpoint `GET /bbs/files/{*path}` for reading file content
  - CLI `bbs get` now includes file search results (type `file::*`)
  - Searches R2-synced content (bridges, imprints, daily notes) on float-box
  - Uses WalkDir for recursive directory traversal
  - Fuzzy matches on filename and extracts title from YAML frontmatter

- **BBS Get Configurable Search Types**
  - `get_search_types` option in `[bbs]` config section
  - Configure which types (inbox, memory, board) to search by default
  - `--type` flag still overrides config
  - Accepts aliases: "memories" for "memory", "boards" for "board"

- **BBS R2 Bucket Fetch**
  - `floatctl bbs get` now searches R2 bucket for archived content
  - Integrated with local search results

- **Board Post --board Flag**
  - `floatctl bbs board post --board <name>` for simpler board targeting
  - Shorthand `-b` flag also available

- **Telemetry in Quick Deploy**
  - Quick deploy script now builds with `--features telemetry`

## [0.2.0] - 2025-12-14

### Added

- **BBS Inbox Show & Fuzzy Get Commands**
  - `floatctl bbs show <id>` - Display full inbox message content
  - `floatctl bbs get <query>` - Fuzzy search across inbox, memories, and boards
  - Server endpoint `GET /:persona/inbox/:id` for single message fetch
  - Auto-displays full content when exactly one match found
  - Type filtering with `--type inbox|memory|board`
  - Configurable result limit with `-n` (default: 5)
  - Tracing instrumentation for telemetry

- **Script Management Commands**
  - `floatctl script register` - Register shell scripts for quick reuse
  - `floatctl script list` - List all registered scripts
  - `floatctl script run` - Run registered scripts with argument passthrough
  - Scripts stored in `~/.floatctl/scripts/` directory
  - Security features: symlink protection, shebang validation (Unix), extension-based execution (Windows)
  - Unit tests for script validation and cross-platform compatibility

- **Claude Code Session Log Querying** (`floatctl-claude` crate)
  - New `floatctl claude` command suite for evna integration
  - `floatctl claude list-sessions` - List recent Claude Code sessions from `~/.claude/projects/`
  - `floatctl claude recent-context` - Extract recent context for system prompt injection (evna's primary use case)
  - `floatctl claude show` - Pretty-print session logs with formatted output
  - JSONL streaming parser for Claude Code history files
  - Handles both user and API message formats
  - Security hardened: uses `execFile()` instead of shell execution in evna integration

- **Bridge Maintenance Operations** (`floatctl-bridge` crate)
  - `floatctl bridge index` - Index `::` annotations from markdown files to create bridge stubs
  - `floatctl bridge append` - Append conversation content to bridge files
  - Supports project, issue, LF1M, and meeting annotation types
  - Smart duplicate detection and content extraction

- **Global Installation Support**
  - Default output directory: `~/.floatctl/conversation-exports`
  - Configuration directory: `~/.floatctl/`
  - Works from any directory without specifying `--out`
  - Auto-creates directories as needed
  - `INSTALL.md` with comprehensive installation guide

- **TOML Configuration System**
  - User config: `~/.floatctl/config.toml` for global defaults
  - Project config: `./floatctl.toml` for project-specific overrides
  - Configuration priority: CLI args → project config → user config → hardcoded defaults
  - `config.toml.example` with all available options documented
  - Support for `~` (tilde) expansion in paths

- **Configurable Options**
  - `general.default_output_dir` - Customize conversation export location
  - `query.default_limit` - Default number of search results (default: 10)
  - `query.threshold` - Similarity threshold for filtering results (optional)
  - `query.output_format` - Default output format: "text" or "json"
  - `embedding.batch_size` - API batch size 1-50 (default: 32)
  - `embedding.rate_limit_ms` - Delay between API calls (default: 500ms)
  - `embedding.skip_existing` - Skip already-embedded messages (default: false)
  - `projects.aliases` - Project name aliases for fuzzy matching

- **Documentation**
  - `docs/config-design.md` - Complete configuration design and future roadmap
  - Enhanced `.env.example` with global vs local installation instructions
  - Updated `README.md` with global installation examples
  - Updated `CLAUDE.md` with configuration system details

- **Tracing and OpenTelemetry Support**
  - `--debug` global flag enables debug logging (sets RUST_LOG=debug)
  - `--otel` global flag enables OpenTelemetry OTLP trace export (requires `--features telemetry`)
  - Feature-gated OpenTelemetry dependencies to keep default binary lean
  - `#[instrument]` spans on key functions across core, embed, and search crates
  - Graceful fallback to console-only logging when OTLP collector unavailable
  - Environment variables: `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`
  - New module: `floatctl-cli/src/tracing_setup.rs`

### Changed

- **Binary Renamed** from `floatctl-cli` to `floatctl` for cleaner UX
  - Installation still uses: `cargo install --path floatctl-cli --features embed`
  - But the installed binary is now simply: `floatctl`

- **CLI Arguments Made Optional**
  - `--out` argument now optional for `split`, `explode`, `full-extract` commands
  - `--limit` argument now optional for `query` command (uses config default)
  - `--batch-size`, `--rate-limit-ms`, `--skip-existing` now optional for `embed` command
  - All optional arguments fall back to TOML config or hardcoded defaults

- **Environment Variable Loading**
  - Multi-location `.env` file support with priority:
    1. Current directory `.env` (highest priority)
    2. `~/.floatctl/.env` (global defaults)
    3. Already-set environment variables (lowest priority)
  - Logs which configuration files were loaded for transparency

### Fixed

- **Security: Command Injection Vulnerability** in `evna-next/src/lib/db.ts`
  - Replaced `exec()` with `execFile()` to eliminate shell interpretation
  - User input now passed as separate arguments, not string interpolation
  - Added security features: 60-second timeout, `windowsHide` flag
  - Prevents exploitation via shell metacharacters (backticks, `$()`, semicolons, pipes)

- **Compiler Warning** in `floatctl-embed/src/lib.rs`
  - Removed unused `current_conv_title` variable
  - Simplified code by using title directly instead of cloning

### Technical Details

- Added `dirs` crate (v5.0) to workspace dependencies for home directory resolution
- Added `toml` crate (v0.8) to workspace dependencies for configuration parsing
- New module: `floatctl-embed/src/config.rs` (~370 lines)
  - `FloatctlConfig::load()` - Loads and merges TOML configs
  - `config::load_dotenv()` - Multi-location .env loader
  - `config::get_default_output_dir()` - Resolves output directory with config support
  - Full test coverage: 7 new unit tests

## [0.1.0] - 2025-10-26

### Added

- Initial release of `floatctl-rs` toolchain
- Streaming JSON/NDJSON parser with O(1) memory usage
- Support for Claude and ChatGPT export formats
- Folder-per-conversation organization with artifact extraction
- Multiple output formats: Markdown, JSON, NDJSON
- Optional semantic search via pgvector embeddings
- OpenAI embeddings integration
- Commands: `ndjson`, `split`, `explode`, `full-extract`, `embed`, `query`
- Token-based message chunking (6000 tokens with 200-token overlap)
- Smart IVFFlat index management
- Progress bars with real-time conversation titles
- Marker-based filtering (project, meeting, date ranges)

### Performance

- Process 772MB files in ~7 seconds with <100MB memory usage
- Streaming architecture ensures O(1) memory regardless of file size
- Parallel conversation processing where applicable

[Unreleased]: https://github.com/float-ritual-stack/floatctl-rs/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/float-ritual-stack/floatctl-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/float-ritual-stack/floatctl-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/float-ritual-stack/floatctl-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/float-ritual-stack/floatctl-rs/releases/tag/v0.1.0
