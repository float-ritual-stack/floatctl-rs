# floatctl-server

HTTP server for floatctl providing REST APIs for:
- Board/thread/message management
- Marker-based search (ctx::, project::, etc.)
- Per-persona inbox messaging
- Common scratchpad with TTL
- CLI command proxy (allowlisted)

## Quick Start

```rust
use floatctl_server::{create_pool, run_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_pool("postgres://localhost/floatctl").await?;
    let config = ServerConfig::default();
    run_server(pool, config).await?;
    Ok(())
}
```

## Endpoints

### Health
- `GET /health` - Server health check

### Boards
- `GET /boards` - List boards (paginated)
- `POST /boards` - Create board
- `GET /boards/{name}` - Get board with thread count

### Threads
- `GET /boards/{name}/threads` - List threads for board
- `POST /boards/{name}/threads` - Create thread (with optional first message)
- `GET /threads/{id}` - Get thread

### Messages
- `GET /threads/{id}/messages` - List messages in thread
- `POST /threads/{id}/messages` - Add message (extracts markers)
- `GET /threads?ctx=X&project=Y` - Search threads by markers

### Inbox
- `GET /inbox/{persona}` - List unread messages
- `POST /inbox/{persona}` - Send message to persona
- `DELETE /inbox/{persona}/{id}` - Mark read/delete

Personas: evna, kitty, cowboy, daddy

### Scratchpad
- `GET /common` - List all items
- `POST /common` - Upsert item (with optional TTL)
- `GET /common/{key}` - Get item
- `DELETE /common/{key}` - Delete item

### CLI Proxy (Restricted)
- `POST /cli/{command}` - Execute floatctl command

Allowlist: search, ctx, query, claude

## Configuration

```rust
ServerConfig {
    bind_addr: "127.0.0.1:3030".parse()?,
    cors_permissive: false,  // localhost only by default
}
```

## Security

- CORS: localhost only by default
- CLI proxy: hardcoded allowlist, 30s timeout
- Input validation on all endpoints

## Development

```bash
# Run tests (requires DATABASE_URL)
DATABASE_URL=postgres://localhost/floatctl_test cargo test -p floatctl-server

# Run ignored integration tests
DATABASE_URL=postgres://localhost/floatctl_test cargo test -p floatctl-server -- --ignored
```

See `HTTP_SERVER_SPEC.md` for detailed specifications.
