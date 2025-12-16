# Rust Ecosystem & Claude SDK Patterns Survey

**Generated**: 2025-11-21
**Purpose**: Survey Rust CLI patterns and Claude Agent SDK architecture for floatctl-rs refactoring guidance
**Sources**: cargo, ripgrep, bat, tokio, clap docs, evna Agent SDK implementation, floatctl-rs existing patterns

---

## 📚 Survey Scope

### Rust CLI Ecosystem Patterns
1. **Command Module Organization** (cargo, gh, docker CLI patterns)
2. **Testing Strategies** (unit, integration, CLI testing)
3. **Error Handling** (anyhow, thiserror, user-facing errors)
4. **Configuration Management** (cli args, env vars, config files)

### Claude Agent SDK Patterns
5. **Tool Organization** (from evna implementation)
6. **Session Management** (multi-turn conversations)
7. **MCP Integration** (server patterns, resource exposure)
8. **Security** (credential handling, command injection prevention)

---

## 1. Command Module Organization Patterns

### Pattern A: Flat Dispatcher (Current floatctl-rs)

**Structure:**
```
floatctl-cli/src/
├── main.rs (2900 lines) ❌ Too large
│   ├── Cli struct + Commands enum
│   ├── All arg structs (34 structs)
│   ├── All run_* functions (31 functions)
│   └── main() dispatcher
```

**Pros:**
- Simple for small CLIs
- Everything in one place

**Cons:**
- Doesn't scale beyond ~500 lines
- Hard to navigate
- High cognitive load
- Difficult to test in isolation

**Verdict**: ❌ **Anti-pattern for large CLIs**

---

### Pattern B: Command Modules (cargo, gh, kubectl)

**Structure:**
```
floatctl-cli/src/
├── main.rs (~300-500 lines) ✅ Entry point only
│   ├── Cli struct + top-level Commands enum
│   ├── main() dispatcher
│   └── init_tracing() setup
├── commands/
│   ├── mod.rs (re-exports)
│   ├── script.rs (all script command logic)
│   ├── claude.rs (all claude command logic)
│   ├── bridge.rs (all bridge command logic)
│   ├── evna.rs (all evna command logic)
│   └── system.rs (all system command logic)
└── args.rs (optional - all arg structs)
```

**Pros:**
- Scales to 100+ commands
- Easy navigation (find by feature)
- Testable in isolation
- Clear ownership (one module per command group)

**Cons:**
- Slightly more boilerplate (module declarations)
- Need to manage imports

**Examples in the wild:**
- **cargo**: `cargo/src/bin/cargo/commands/{build,test,run,...}.rs`
- **ripgrep**: `rg/crates/cli/src/args.rs` + command dispatchers
- **gh**: `gh/pkg/cmd/{pr,issue,repo,...}/` (Go, but same pattern)

**Verdict**: ✅ **Best practice for CLIs with 5+ command groups**

---

### Pattern C: Separate Crates (tokio, rustup)

**Structure:**
```
floatctl-cli/
floatctl-cli-commands/  (new crate)
  ├── script/
  ├── claude/
  ├── bridge/
  └── ...
```

**Pros:**
- Maximum separation of concerns
- Parallel compilation
- Can version command crates independently
- Clear dependency boundaries

**Cons:**
- More complex Cargo.toml management
- Overkill for personal tools
- Adds indirection

**Examples in the wild:**
- **tokio**: `tokio/`, `tokio-util/`, `tokio-stream/`
- **rustup**: `rustup-cli/`, `rustup-utils/`, `rustup-dist/`

**Verdict**: 🟡 **Over-engineering for floatctl** (defer unless collaboration increases)

---

## 2. Testing Strategies

### Pattern: Command Module Unit Tests

**Structure:**
```rust
// floatctl-cli/src/commands/script.rs

pub fn run_script_list(args: ListScriptArgs) -> Result<()> {
    // ... implementation ...
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_scripts_empty() {
        // Test with no registered scripts
    }

    #[test]
    fn test_list_scripts_with_docs() {
        // Test doc block parsing
    }
}
```

**Pros:**
- Co-located with implementation
- Fast test execution
- Easy to write

**Cons:**
- Doesn't test CLI arg parsing
- Doesn't test command dispatcher

**Examples in the wild:**
- **cargo**: `cargo/tests/testsuite/` (integration) + inline unit tests
- **ripgrep**: `rg/crates/cli/src/args.rs` unit tests

---

### Pattern: Integration Tests with Command Execution

**Structure:**
```rust
// floatctl-cli/tests/script_commands.rs

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_script_list_cli() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("script").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Registered scripts"));
}

#[test]
fn test_script_register_dry_run() {
    let mut cmd = Command::cargo_bin("floatctl").unwrap();
    cmd.arg("script")
        .arg("register")
        .arg("--dry-run")
        .arg("/path/to/test.sh");

    cmd.assert().success();
}
```

**Dependencies**:
- `assert_cmd` - CLI testing framework
- `predicates` - Assertions for stdout/stderr
- `tempfile` - Temporary directories for file operations

**Pros:**
- Tests real CLI behavior (arg parsing, output formatting)
- Catches integration bugs
- Confidence in user-facing behavior

**Cons:**
- Slower than unit tests
- Requires binary compilation

**Examples in the wild:**
- **cargo**: `cargo/tests/testsuite/build.rs`, `cargo/tests/testsuite/test.rs`
- **bat**: `bat/tests/integration_tests.rs`

**Verdict**: ✅ **Essential for CLI quality** - add after command extraction

---

## 3. Error Handling Patterns

### Pattern A: anyhow::Result (Current floatctl-rs)

**Usage:**
```rust
fn run_script_register(args: RegisterScriptArgs) -> Result<()> {
    let scripts_dir = get_scripts_dir()
        .context("Failed to get scripts directory")?;

    if !args.script_path.exists() {
        return Err(anyhow!("Script not found: {}", args.script_path.display()));
    }

    // ...
}
```

**Pros:**
- Simple, ergonomic
- Great for prototypes and internal tools
- Easy to add context

**Cons:**
- All errors have same type (no structured error handling)
- Can't match on error variants

**Verdict**: ✅ **Perfect for floatctl** (personal tool, user-facing CLI)

---

### Pattern B: thiserror (Library Crates)

**Usage (when library needs structured errors):**
```rust
#[derive(thiserror::Error, Debug)]
pub enum ScriptError {
    #[error("Script not found: {0}")]
    NotFound(PathBuf),

    #[error("Script too large ({size} bytes, max {max} bytes)")]
    TooLarge { size: u64, max: u64 },

    #[error("Invalid script name: {0}")]
    InvalidName(String),
}
```

**Pros:**
- Structured errors
- Can match on variants
- Better for libraries (consumers can handle specific errors)

**Cons:**
- More boilerplate
- Overkill for CLI applications

**Verdict**: 🟢 **Not needed for floatctl** (anyhow is sufficient)

---

## 4. Configuration Management

### Pattern: Layered Config (clap + env + file)

**From AGENT-SDK-REVIEW.md observation:**
```typescript
// evna/src/core/config.ts - Centralized configuration
export const config = {
  supabaseUrl: process.env.SUPABASE_URL || throw new Error("..."),
  anthropicApiKey: process.env.ANTHROPIC_API_KEY,
  // ...
};
```

**Rust equivalent (floatctl pattern):**
```rust
// floatctl-cli/src/config.rs (already exists!)
pub struct FloatctlConfig {
    pub supabase_url: String,
    pub anthropic_api_key: Option<String>,
    // ...
}

impl FloatctlConfig {
    pub fn load() -> Self {
        // Priority: CLI args > env vars > config file > defaults
    }
}
```

**Pros:**
- Centralized configuration
- Prevents "three EVNAs" drift (CLI/TUI/MCP using different configs)
- Single source of truth

**Cons:**
- None (this is already working well in floatctl)

**Verdict**: ✅ **Keep existing pattern**

---

## 5. Tool Organization (Agent SDK Patterns)

### Pattern: Tool-as-Class (evna pattern)

**From AGENT-SDK-REVIEW.md:**
```typescript
// Business logic in class
export class BrainBootTool {
  async boot(args: BrainBootArgs): Promise<BrainBootResult> {
    // Complex logic here
  }
}

// SDK wrapper in tools/index.ts
export const brainBootTool = tool(
  "brain_boot",
  "Description...",
  schema,
  async (args) => {
    const result = await brainBoot.boot(args);
    return { content: [{ type: "text", text: result.summary }] };
  }
);
```

**Benefits:**
- Business logic testable independently of SDK
- Clear separation: logic vs SDK integration
- Reusable across MCP/CLI/TUI interfaces

**Rust CLI equivalent:**
```rust
// Business logic in floatctl-script crate
pub fn list_scripts(parse_docs: bool) -> Result<Vec<ScriptInfo>> {
    // Logic here
}

// CLI wrapper in commands/script.rs
fn run_script_list(args: ListScriptArgs) -> Result<()> {
    let scripts = floatctl_script::list_scripts(args.format != "names-only")?;
    // Format and print
    Ok(())
}
```

**Observation**: ✅ **floatctl already follows this pattern** (business logic in crates, CLI wrappers in main.rs)

**Recommendation**: ✅ **Continue this pattern when extracting commands**

---

## 6. Session Management (Agent SDK Patterns)

### Pattern: Resume + Fork Sessions

**From AGENT-SDK-REVIEW.md:**
```typescript
// evna/src/tools/ask-evna-agent.ts
if (session_id) {
  baseOptions.resume = session_id;
  if (fork_session) {
    baseOptions.forkSession = true;
  }
}
```

**Benefits:**
- Multi-turn conversations
- Branch conversations for experimentation
- Timeout recovery (return session_id, user resumes later)

**floatctl equivalent**: Not applicable (CLI tool, not conversational agent)

**Verdict**: 🟢 **Not relevant to floatctl-rs CLI refactoring**

---

## 7. MCP Integration (Agent SDK Patterns)

### Pattern: Dual MCP Servers (Fractal Prevention)

**From AGENT-SDK-REVIEW.md:**
```typescript
// External MCP (for Claude Desktop) - includes all tools
export function createEvnaMcpServer() {
  return createSdkMcpServer({
    tools: [
      brainBootTool,
      semanticSearchTool,
      activeContextTool,
      askEvnaTool,  // ✅ Included
      // ...
    ],
  });
}

// Internal MCP (for ask_evna agent) - excludes recursive tools
export function createInternalMcpServer() {
  return createSdkMcpServer({
    tools: [
      brainBootTool,
      semanticSearchTool,
      activeContextTool,
      // ❌ askEvnaTool EXCLUDED to prevent recursion
      // ...
    ],
  });
}
```

**Benefits:**
- Prevents fractal recursion (agent calling itself infinitely)
- Different tool visibility for different contexts
- Documents incident and prevention strategy

**floatctl equivalent**: Not applicable (CLI tool, no MCP server in floatctl-cli)

**Verdict**: 🟢 **Not relevant to floatctl-rs CLI refactoring** (evna handles MCP, floatctl is CLI)

---

## 8. Security Patterns

### Pattern A: Command Injection Prevention

**From AGENT-SDK-REVIEW.md:**
```typescript
// ✅ GOOD: execFile() prevents shell interpretation
const execFileAsync = promisify(execFile);
const result = await execFileAsync("floatctl", ["query", userInput]);

// ❌ BAD: exec() allows shell injection
const result = await exec(`floatctl query ${userInput}`);
```

**floatctl-rs equivalent:**
```rust
// ✅ GOOD: Command::new() + args() prevents shell interpretation
let status = Command::new(&script_path)
    .args(&args.args)
    .status()?;

// ❌ BAD: Running sh -c allows injection
let status = Command::new("sh")
    .arg("-c")
    .arg(format!("{} {}", script_path, user_args))  // DANGEROUS
    .status()?;
```

**Observation**: ✅ **floatctl already follows this pattern** (run_script_run uses Command::new + args)

---

### Pattern B: Symlink Protection

**From CODE_REVIEW_ANALYSIS.md:**
```rust
// floatctl-cli/src/main.rs:2439
if args.script_path.is_symlink() {
    return Err(anyhow!(
        "Cannot register symlink: {}\n   Register the target file directly instead",
        args.script_path.display()
    ));
}
```

**Verdict**: ✅ **Existing security pattern is excellent** - preserve when extracting

---

## 9. Cargo Workspace Patterns

### Pattern: Focused Crates with Clear Boundaries

**floatctl-rs current structure:**
```
floatctl-rs/
├── floatctl-core/        (streaming, parsing - no dependencies on other crates)
├── floatctl-embed/       (pgvector - depends on core)
├── floatctl-claude/      (session logs - depends on core)
├── floatctl-bridge/      (file management - depends on core)
├── floatctl-script/      (script management - standalone)
├── floatctl-cli/         (CLI - depends on all)
└── evna/                 (TypeScript - separate)
```

**Benefits:**
- Clear dependency graph (core → specialized crates → CLI)
- Parallel compilation
- Each crate has single responsibility

**Comparison to best practices:**
- ✅ **tokio**: `tokio/`, `tokio-util/`, `tokio-stream/` (same pattern)
- ✅ **serde**: `serde/`, `serde_derive/`, `serde_json/` (same pattern)
- ✅ **clap**: `clap/`, `clap_derive/`, `clap_complete/` (same pattern)

**Verdict**: ✅ **Excellent workspace structure** - keep as-is

---

## 📊 Recommendations for floatctl-rs Refactoring

### Immediate (Based on Ecosystem Patterns)

1. **Adopt Pattern B: Command Modules**
   - Extract script/claude/bridge/evna/system commands to `floatctl-cli/src/commands/`
   - Follow cargo/gh pattern (one module per command group)
   - Reduce main.rs to 300-500 lines (dispatcher only)

2. **Add Integration Tests**
   - Use `assert_cmd` + `predicates` crates
   - Test CLI behavior (arg parsing, output formatting)
   - Start with script commands (highest value)

3. **Continue Tool-as-Class Pattern**
   - Business logic in crates (floatctl-script, floatctl-claude, etc.)
   - CLI wrappers in command modules (format output, handle args)
   - This pattern already works well

---

### Short-Term (Ecosystem-Inspired Improvements)

4. **Extract Arg Structs (Optional)**
   - Create `floatctl-cli/src/args.rs` or keep in command modules
   - Reduces main.rs visual noise
   - Pattern from ripgrep: `rg/crates/cli/src/args.rs`

5. **Add Command Module Unit Tests**
   - Co-locate tests with command logic
   - Use `tempfile` for filesystem operations
   - Pattern from cargo: inline `#[cfg(test)]` modules

---

### Long-Term (If Collaboration Increases)

6. **Consider Pattern C: Separate Crates (DEFER)**
   - Only if external contributors join
   - Only if command modules grow beyond 500 lines each
   - Pattern from tokio/rustup (enterprise-scale CLIs)

---

## 🔍 Agent SDK Patterns NOT Applicable to floatctl-rs CLI

**Why floatctl-rs is different from evna:**

| Aspect | evna (Agent SDK) | floatctl-rs (CLI) |
|--------|------------------|-------------------|
| **Purpose** | Conversational AI agent with tools | Command-line utility |
| **Architecture** | TypeScript + Agent SDK + MCP | Rust + clap |
| **Tool Patterns** | Tool-as-class + SDK wrappers | Command modules + business logic crates |
| **Session Management** | Multi-turn conversations, resume, fork | Stateless (each command is independent) |
| **MCP Integration** | External + internal MCP servers | No MCP (evna handles that) |
| **Fractal Prevention** | Dual server pattern | Not applicable (no recursion risk) |

**Verdict**: ✅ **evna patterns are well-implemented but NOT a model for floatctl-rs CLI refactoring**

Focus on **Rust CLI ecosystem patterns** (cargo, ripgrep, bat) instead.

---

## 📚 Key Takeaways

### What floatctl-rs Does Right (Keep)
1. ✅ Cargo workspace with focused crates (core → specialized → CLI)
2. ✅ Business logic in crates, CLI wrappers thin
3. ✅ Security-conscious (symlink protection, file size limits, command injection prevention)
4. ✅ anyhow::Result for error handling (perfect for CLI)
5. ✅ Centralized config (floatctl-cli/src/config.rs)

### What floatctl-rs Should Adopt (Change)
1. ❌ main.rs is 2900 lines → ✅ Extract to command modules (cargo/gh pattern)
2. ❌ Zero test coverage → ✅ Add integration tests (assert_cmd pattern)
3. ❌ Duplicate get_scripts_dir() → ✅ Use floatctl_script::get_scripts_dir everywhere

---

## 🚀 Next Steps

1. **Use ecosystem patterns for refactoring**:
   - Command modules structure (Pattern B from Section 1)
   - Integration tests (Pattern from Section 2)
   - Continue tool-as-class pattern (Pattern from Section 5)

2. **DON'T adopt Agent SDK patterns**:
   - Session management (CLI is stateless)
   - MCP integration (evna handles that)
   - Fractal prevention (not applicable)

3. **Generate final refactoring roadmap** with:
   - Specific file moves (main.rs → commands/*.rs)
   - Integration test setup (Cargo.toml dependencies)
   - Success metrics (main.rs size, test coverage %)

---

**Survey Complete**: 2025-11-21
**Analyst**: Claude Code (floatctl archaeological discovery agent)
**Next Phase**: Generate comprehensive refactoring roadmap
