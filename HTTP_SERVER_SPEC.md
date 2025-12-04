# FLOATCTL HTTP SERVER - SPECIFICATION BLUEPRINT

> Progressive spec-building workflow for the floatctl HTTP server.
> Each phase builds on the previous. Complete tests before proceeding.

## Context

**Existing Stack:**
- PostgreSQL 15+ with pgvector (not SQLite - aligns with existing `floatctl-embed`)
- sqlx 0.8 with compile-time query verification
- Tokio async runtime
- Existing marker system in `floatctl-core/src/markers.rs`

**New Dependencies (Phase 2+):**
- axum 0.7 (HTTP framework)
- tower-http (CORS, tracing middleware)

**Crate:** `floatctl-server` (new workspace member)

---

## Anti-Patterns Checklist

Before any PR, verify:

- [ ] No CLI endpoint without allowlist (if exposing CLI)
- [ ] No N+1 queries (every list uses JOINs)
- [ ] No dynamic SQL string building (use sqlx::QueryBuilder)
- [ ] All user input has max length validation
- [ ] Connection pool, not Arc<Mutex<Connection>>
- [ ] No check-then-insert (use DB constraints)
- [ ] CORS: localhost only (default)

---

## Phase 1: Foundation (No HTTP)

### Spec 1.1: Database Module

**Feature:** Connection pool and schema for boards/threads/messages

**Schema (PostgreSQL):**

```sql
-- Migration: 0010_server_schema.sql

CREATE TABLE IF NOT EXISTS boards (
    name        TEXT PRIMARY KEY CHECK (name ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS threads (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_name  TEXT NOT NULL REFERENCES boards(name) ON DELETE CASCADE,
    title       TEXT NOT NULL CHECK (length(title) <= 256),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS thread_messages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id   UUID NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    content     TEXT NOT NULL CHECK (length(content) <= 65536),
    author      TEXT CHECK (author IS NULL OR length(author) <= 64),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS message_markers (
    message_id  UUID NOT NULL REFERENCES thread_messages(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('ctx', 'project', 'mode', 'bridge', 'float')),
    value       TEXT NOT NULL CHECK (length(value) <= 256),
    PRIMARY KEY (message_id, kind, value)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_threads_board ON threads(board_name);
CREATE INDEX IF NOT EXISTS idx_threads_created ON threads(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON thread_messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON thread_messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_markers_kind_value ON message_markers(kind, value);
```

**Connection Pool Pattern:**

```rust
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)  // Explicit, documented limit
        .connect(database_url)
        .await
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `pool_acquires_connection` | Pool connects, query executes |
| `schema_creates_idempotently` | Running migrations twice succeeds |
| `fk_prevents_orphan_message` | Insert message with invalid thread_id fails |
| `concurrent_pool_access` | 10 concurrent tasks all succeed |
| `board_name_constraint` | Invalid slug (e.g., "UPPER", "a b c") rejected by DB |

---

### Spec 1.2: Domain Models

**Feature:** Type-safe models with validation at construction

**Models:**

```rust
// Board name: slug format, max 64 chars
pub struct BoardName(String);

impl BoardName {
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^[a-z0-9][a-z0-9_-]{0,63}$").unwrap()
        });
        if RE.is_match(s) {
            Ok(Self(s.to_owned()))
        } else {
            Err(ValidationError::InvalidSlug { value: s.to_owned() })
        }
    }
}

// Thread title: max 256 chars, non-empty
pub struct ThreadTitle(String);

impl ThreadTitle {
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            Err(ValidationError::Empty { field: "title" })
        } else if trimmed.len() > 256 {
            Err(ValidationError::TooLong { field: "title", max: 256 })
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }
}

// Message content: max 64KB
pub struct MessageContent(String);

impl MessageContent {
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        if s.len() > 65536 {
            Err(ValidationError::TooLong { field: "content", max: 65536 })
        } else {
            Ok(Self(s.to_owned()))
        }
    }
}

// Marker: reuse existing floatctl-core MarkerSet extraction
pub enum MarkerKind { Ctx, Project, Mode, Bridge, Float }

pub struct Marker {
    pub kind: MarkerKind,
    pub value: String,
}
```

**Integration with floatctl-core:**

```rust
use floatctl_core::markers::extract_markers;

impl MessageContent {
    pub fn extract_markers(&self) -> Vec<Marker> {
        extract_markers(&self.0)
            .into_iter()
            .filter_map(|m| Marker::parse(&m))
            .collect()
    }
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `board_name_valid_slug` | "my-board-123" succeeds |
| `board_name_rejects_uppercase` | "MyBoard" returns ValidationError |
| `board_name_rejects_spaces` | "my board" returns ValidationError |
| `board_name_max_length` | 65 chars rejected, 64 chars accepted |
| `thread_title_rejects_empty` | "" and "   " both fail |
| `thread_title_rejects_long` | 257 chars fails, 256 succeeds |
| `message_content_max_64kb` | 65537 bytes fails, 65536 succeeds |
| `marker_extraction_ctx` | "ctx::review" extracts Ctx("review") |
| `marker_extraction_multiple` | "ctx::a project::b" extracts both |

---

### Spec 1.3: CRUD Operations

**Feature:** Database CRUD with proper patterns (no N+1, transactions)

**Repository Pattern:**

```rust
pub struct BoardRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> BoardRepo<'a> {
    /// Creates board, returns existing on conflict (idempotent)
    pub async fn create(&self, name: BoardName) -> Result<Board, DbError> {
        // Use ON CONFLICT DO NOTHING + SELECT
        // Single query, not check-then-insert
    }

    /// List boards with thread counts (single JOIN query)
    pub async fn list(&self, page: Pagination) -> Result<Vec<BoardWithCount>, DbError> {
        // SELECT b.*, COUNT(t.id) FROM boards b LEFT JOIN threads t ...
        // GROUP BY b.name ORDER BY b.created_at LIMIT $1 OFFSET $2
    }
}

pub struct ThreadRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ThreadRepo<'a> {
    /// Creates thread with optional first message (transaction)
    pub async fn create_with_message(
        &self,
        board: BoardName,
        title: ThreadTitle,
        first_message: Option<(MessageContent, Option<String>)>,
    ) -> Result<Thread, DbError> {
        let mut tx = self.pool.begin().await?;
        // 1. Insert thread
        // 2. If first_message, insert message + markers
        // 3. Commit or rollback
        tx.commit().await?;
    }
}

pub struct MessageRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MessageRepo<'a> {
    /// Paginated messages for thread
    pub async fn list_for_thread(
        &self,
        thread_id: Uuid,
        page: Pagination,
    ) -> Result<Paginated<Message>, DbError> {
        // COUNT(*) + SELECT with LIMIT/OFFSET in one round-trip
        // Use window function: SELECT *, COUNT(*) OVER() as total
    }
}
```

**Pagination:**

```rust
pub struct Pagination {
    pub page: u32,      // 1-indexed
    pub per_page: u32,  // max 100
}

impl Pagination {
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 100),
        }
    }

    pub fn offset(&self) -> i64 {
        ((self.page - 1) * self.per_page) as i64
    }
}

pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `list_boards_single_query` | With 100 boards, sqlx logs show exactly 1 query |
| `create_board_idempotent` | Creating same board twice returns same record, no error |
| `create_thread_transaction` | If message insert fails, thread is NOT created |
| `pagination_offset_math` | page=1, per_page=10 → offset=0; page=2 → offset=10 |
| `pagination_clamps_values` | per_page=999 clamped to 100 |
| `list_messages_with_total` | Returns total count for pagination UI |

---

## Phase 2: HTTP Layer

### Spec 2.1: Server Skeleton

**Feature:** Axum server with middleware

**Server Setup:**

```rust
use axum::{Router, routing::get};
use tower_http::cors::{CorsLayer, AllowOrigin};
use std::net::SocketAddr;

pub struct ServerConfig {
    pub bind_addr: SocketAddr,  // Default: 127.0.0.1:3030
    pub cors_permissive: bool,  // Default: false (localhost only)
}

pub async fn run_server(pool: PgPool, config: ServerConfig) -> Result<(), ServerError> {
    let cors = if config.cors_permissive {
        CorsLayer::permissive()  // Documented security tradeoff
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|origin, _| {
                origin.as_bytes().starts_with(b"http://localhost")
                    || origin.as_bytes().starts_with(b"http://127.0.0.1")
            }))
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(AppState { pool });

    let listener = TcpListener::bind(config.bind_addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm.recv() => {},
    }
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `server_starts_and_responds` | GET /health returns 200 |
| `cors_rejects_external` | Origin: https://evil.com → no CORS headers |
| `cors_allows_localhost` | Origin: http://localhost:3000 → CORS headers present |
| `graceful_shutdown` | Send SIGTERM, in-flight request completes |

---

### Spec 2.2: Board Endpoints

**Feature:** Board REST endpoints

**Routes:**

```
GET  /boards              → list_boards (paginated)
POST /boards              → create_board
GET  /boards/{name}       → get_board (with thread count)
```

**Handlers:**

```rust
async fn list_boards(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<BoardWithCount>>, ApiError> {
    let page = Pagination::new(params.page.unwrap_or(1), params.per_page.unwrap_or(20));
    let boards = BoardRepo::new(&state.pool).list(page).await?;
    Ok(Json(boards))
}

#[derive(Deserialize)]
struct CreateBoardRequest {
    name: String,  // Validated → BoardName
}

async fn create_board(
    State(state): State<AppState>,
    Json(req): Json<CreateBoardRequest>,
) -> Result<(StatusCode, Json<Board>), ApiError> {
    let name = BoardName::new(&req.name)?;  // 400 if invalid
    let board = BoardRepo::new(&state.pool).create(name).await?;
    Ok((StatusCode::CREATED, Json(board)))
}
```

**Error Mapping:**

```rust
pub enum ApiError {
    Validation(ValidationError),
    NotFound { resource: &'static str, id: String },
    Database(DbError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::Validation(e) => (StatusCode::BAD_REQUEST, json!({ "error": e.to_string() })),
            Self::NotFound { resource, id } => (
                StatusCode::NOT_FOUND,
                json!({ "error": format!("{} '{}' not found", resource, id) })
            ),
            Self::Database(e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": "internal error" }))
            }
        };
        (status, Json(body)).into_response()
    }
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `get_boards_paginated` | ?page=2&per_page=5 returns correct slice |
| `post_board_invalid_name` | "BAD NAME" → 400 with validation message |
| `post_board_idempotent` | Creating same board twice → 201 both times, same data |
| `get_board_not_found` | /boards/nonexistent → 404 with JSON error |
| `get_board_with_count` | Board with 3 threads shows thread_count: 3 |

---

### Spec 2.3: Thread + Message Endpoints

**Feature:** Thread and message CRUD

**Routes:**

```
GET  /boards/{name}/threads           → list_threads
POST /boards/{name}/threads           → create_thread
GET  /threads/{id}/messages           → list_messages
POST /threads/{id}/messages           → create_message
```

**Create Thread Request:**

```rust
#[derive(Deserialize)]
struct CreateThreadRequest {
    title: String,
    first_message: Option<FirstMessageRequest>,
}

#[derive(Deserialize)]
struct FirstMessageRequest {
    content: String,
    author: Option<String>,
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `create_thread_atomic` | Thread + message created or neither |
| `create_thread_on_missing_board` | /boards/nonexistent/threads → 404 |
| `message_extracts_markers` | Content "ctx::review project::api" → markers stored |
| `list_messages_paginated` | 100 messages, page 3 of 10 → correct offset |
| `add_message_to_missing_thread` | POST /threads/{bad-uuid}/messages → 404 |

---

## Phase 3: Float-Specific Features

### Spec 3.1: Marker Search

**Feature:** Filter threads/messages by marker

**Schema Addition:**

```sql
-- Already covered by message_markers table + index
```

**Routes:**

```
GET /threads?project=api&ctx=review   → filtered threads
GET /messages?ctx=review              → filtered messages (across all threads)
```

**Query Logic:**

```rust
// Multiple markers = AND (intersection)
pub async fn search_threads_by_markers(
    pool: &PgPool,
    filters: &[(MarkerKind, &str)],
    page: Pagination,
) -> Result<Paginated<Thread>, DbError> {
    // Build query with EXISTS subqueries for each marker
    // SELECT t.* FROM threads t
    // WHERE EXISTS (SELECT 1 FROM message_markers m
    //               JOIN thread_messages tm ON m.message_id = tm.id
    //               WHERE tm.thread_id = t.id AND m.kind = $1 AND m.value = $2)
    //   AND EXISTS (...)  -- for each filter
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `filter_single_marker` | ?project=api returns only matching threads |
| `filter_multiple_markers` | ?project=api&ctx=review → intersection |
| `filter_no_matches` | ?project=nonexistent → empty list, not error |
| `filter_uses_index` | EXPLAIN shows idx_markers_kind_value used |

---

### Spec 3.2: Inbox (Per-Persona Messaging)

**Feature:** Async message inbox for personas

**Schema:**

```sql
CREATE TABLE IF NOT EXISTS inboxes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    persona     TEXT NOT NULL CHECK (persona IN ('evna', 'kitty', 'cowboy', 'daddy')),
    content     TEXT NOT NULL CHECK (length(content) <= 65536),
    from_persona TEXT CHECK (from_persona IS NULL OR from_persona IN ('evna', 'kitty', 'cowboy', 'daddy')),
    read_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_inbox_persona ON inboxes(persona, created_at DESC);
```

**Routes:**

```
GET    /inbox/{persona}       → list unread messages
POST   /inbox/{persona}       → send message to persona
DELETE /inbox/{persona}/{id}  → mark read / delete
```

**Persona Enum:**

```rust
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Persona {
    Evna,
    Kitty,
    Cowboy,
    Daddy,
}

impl Persona {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Evna => "evna",
            Self::Kitty => "kitty",
            Self::Cowboy => "cowboy",
            Self::Daddy => "daddy",
        }
    }
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `invalid_persona_400` | /inbox/invalid → 400, not 500 |
| `inbox_isolation` | kitty's inbox doesn't show cowboy's messages |
| `delete_idempotent` | DELETE same message twice → 204 both times |
| `list_excludes_read` | Messages with read_at not in default list |

---

### Spec 3.3: Common Scratchpad

**Feature:** Shared key-value store with optional TTL

**Schema:**

```sql
CREATE TABLE IF NOT EXISTS scratchpad (
    key         TEXT PRIMARY KEY CHECK (length(key) <= 256),
    value       JSONB NOT NULL,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scratchpad_expires ON scratchpad(expires_at) WHERE expires_at IS NOT NULL;
```

**Routes:**

```
GET    /common           → list all (non-expired) items
POST   /common           → upsert key-value
GET    /common/{key}     → get single item
DELETE /common/{key}     → delete item
```

**TTL Logic:**

```rust
// On any read, clean expired items (lazy cleanup)
pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, DbError> {
    sqlx::query!("DELETE FROM scratchpad WHERE expires_at < NOW()")
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
}

// Called at start of list/get handlers (non-blocking spawn)
fn spawn_cleanup(pool: PgPool) {
    tokio::spawn(async move {
        let _ = cleanup_expired(&pool).await;
    });
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `ttl_expiration` | Item with ttl_seconds=1 gone after 2s |
| `upsert_behavior` | POST same key twice updates value |
| `cleanup_non_blocking` | Request returns before cleanup finishes |
| `value_is_jsonb` | Can store objects, arrays, not just strings |

---

## Phase 4: CLI Bridge (Optional - High Risk)

### Spec 4.1: CLI Command Proxy

**Feature:** Execute allowlisted floatctl commands via HTTP

**SECURITY MODEL:**

```rust
// Hard-coded allowlist - NOT configurable
const ALLOWED_COMMANDS: &[&str] = &["search", "ctx", "query", "claude"];

// Explicitly blocked (even if someone tries to add)
const BLOCKED_COMMANDS: &[&str] = &["server", "embed", "sync"];
```

**Routes:**

```
POST /cli/{command}   → execute command
```

**Invoker Trait (for testing):**

```rust
#[async_trait]
pub trait CliInvoker: Send + Sync {
    async fn invoke(&self, command: &str, args: Vec<String>) -> Result<Output, InvokeError>;
}

pub struct RealInvoker;

#[async_trait]
impl CliInvoker for RealInvoker {
    async fn invoke(&self, command: &str, args: Vec<String>) -> Result<Output, InvokeError> {
        let output = tokio::process::Command::new("floatctl")
            .arg(command)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        Ok(Output {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

// For tests
pub struct MockInvoker { /* ... */ }
```

**Timeout Enforcement:**

```rust
const CLI_TIMEOUT: Duration = Duration::from_secs(30);

async fn execute_cli(
    invoker: &dyn CliInvoker,
    command: &str,
    args: Vec<String>,
) -> Result<Output, CliError> {
    match tokio::time::timeout(CLI_TIMEOUT, invoker.invoke(command, args)).await {
        Ok(result) => result.map_err(CliError::Invoke),
        Err(_) => Err(CliError::Timeout { seconds: 30 }),
    }
}
```

**Tests Required:**

| Test | Assertion |
|------|-----------|
| `allowed_command_executes` | POST /cli/search with args succeeds |
| `disallowed_command_403` | POST /cli/embed → 403 Forbidden |
| `blocked_command_403` | POST /cli/server → 403 (explicit block) |
| `timeout_kills_process` | Slow command killed after 30s |
| `mock_invoker_testable` | Unit test without subprocess |
| `args_not_shell_interpolated` | Args with spaces/special chars handled safely |

---

## Implementation Checklist

### Per-Spec Deliverables

For each spec, deliver:

1. **Types/traits** in `floatctl-server/src/`
2. **Implementation** with proper error handling
3. **Unit tests** in same file or `tests/` directory
4. **Integration tests** using test database
5. **Migration SQL** in `/migrations/` (if schema change)
6. **README update** (if public API change)

### PR Checklist

Before merging:

- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] No N+1 queries (verified with query logging)
- [ ] All input validated at HTTP boundary
- [ ] Error responses are JSON, not panics
- [ ] CORS tested with curl

---

## Crate Structure

```
floatctl-server/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public exports
│   ├── db/
│   │   ├── mod.rs
│   │   ├── pool.rs         # Connection pool
│   │   └── repos/
│   │       ├── mod.rs
│   │       ├── boards.rs
│   │       ├── threads.rs
│   │       ├── messages.rs
│   │       ├── inbox.rs
│   │       └── scratchpad.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── board.rs
│   │   ├── thread.rs
│   │   ├── message.rs
│   │   ├── marker.rs
│   │   ├── persona.rs
│   │   └── validation.rs
│   ├── http/
│   │   ├── mod.rs
│   │   ├── server.rs       # Axum setup
│   │   ├── error.rs        # ApiError IntoResponse
│   │   ├── extractors.rs   # Custom extractors
│   │   └── routes/
│   │       ├── mod.rs
│   │       ├── health.rs
│   │       ├── boards.rs
│   │       ├── threads.rs
│   │       ├── messages.rs
│   │       ├── inbox.rs
│   │       ├── scratchpad.rs
│   │       └── cli.rs
│   └── cli/
│       ├── mod.rs
│       └── invoker.rs      # CliInvoker trait
├── tests/
│   ├── db_integration.rs
│   └── api_integration.rs
└── README.md
```

---

## Quick Reference: Lessons Learned

| Problem | Solution |
|---------|----------|
| N+1 queries | JOINs with COUNT, window functions |
| No input validation | Newtypes with validation in `new()` |
| CLI injection risk | Allowlist + timeout + trait for testing |
| Missing indexes | Explicit CREATE INDEX in migrations |
| Race conditions | ON CONFLICT + transactions |
| Permissive CORS | Localhost-only default |
| Check-then-insert | Rely on DB constraints |
| Arc<Mutex<Conn>> | sqlx connection pool |

---

## Next Steps

1. Create `floatctl-server` crate with Cargo.toml
2. Implement Spec 1.1 (database module) with tests
3. Run tests: `cargo test -p floatctl-server`
4. Proceed to Spec 1.2 only after 1.1 tests pass
