# MCP Resource Research & Implementation
**Date**: 2025-10-24
**Project**: float/evna-next
**Context**: ctx::2025-10-24 @ 03:32:12 PM - [project::float/evna]

---

## Research Summary: MCP Feature Support

### Claude Code vs Claude Desktop

| Feature | Claude Code | Claude Desktop | Notes |
|---------|-------------|----------------|-------|
| **Static Resources** | ✅ Full support | ✅ Full support | Both handle fixed URIs |
| **Resource Templates** | ✅ Full support | ⚠️ Limited/Buggy | Desktop has issues (GitHub #263, #1016) |
| **@ Mention Discovery** | ✅ Auto-completion | ⚠️ Manual | Code shows resources in menu |
| **URI Completion** | ✅ Supported | ❓ Unknown | Code supports completion callbacks |
| **Dynamic Discovery** | ✅ Works well | ❌ Broken | Desktop only sees static reliably |
| **Size Limits** | 25K tokens | 1MB | Desktop errors on large resources |

**Conclusion**: Claude Code is the primary target. Use static resources for Desktop compatibility.

---

## URI Design: Conflict-Free Architecture

### Final Decision: Scheme Separation

```
daily://    - Static curated views (today, recent, week, list)
notes://    - Dynamic template (future) for entire vault access
```

**Why this works**:
- Zero URI conflicts (different schemes)
- Clear semantics (curated vs file system)
- Extensible (add more static views without breaking templates)
- Future-proof (bridges://, tldr://, etc.)

### Alternatives Considered

1. **Reserved Keywords** - Fragile, requires maintaining exclusion list
2. **Explicit Namespace** (`daily://date/2025-10-24`) - Works but verbose
3. **Different Schemes** (`daily://` vs `dailies://`) - CHOSEN for clarity

---

## Implementation: `daily://` Resources

### Added Resources (src/mcp-server.ts)

```typescript
resources: [
  {
    uri: "daily://today",
    name: "Today's Daily Note",
    description: "Returns today's daily note (YYYY-MM-DD.md)",
    mimeType: "text/markdown",
  },
  {
    uri: "daily://recent",
    name: "Recent Daily Notes (Last 3 Days)",
    description: "Last 3 days concatenated with date headers",
    mimeType: "text/markdown",
  },
  {
    uri: "daily://week",
    name: "This Week's Daily Notes (Last 7 Days)",
    description: "Last 7 days concatenated with date headers",
    mimeType: "text/markdown",
  },
  {
    uri: "daily://list",
    name: "Available Daily Notes",
    description: "JSON list of last 30 days",
    mimeType: "application/json",
  },
]
```

### Concatenation Format

```markdown
# 2025-10-24

[content of 2025-10-24.md]

---

# 2025-10-23

[content of 2025-10-23.md]

---

# 2025-10-22

[content of 2025-10-22.md]
```

**Missing file handling**: Shows `*(No note found)*` placeholder instead of failing.

### Test Results

```
🚀 MCP Resource Tests

🧪 Testing daily://today
✅ Found today's note (2025-10-24)

🧪 Testing daily://recent
✅ Concatenated 3 days
   Dates: 2025-10-24, 2025-10-23, 2025-10-22
   Total length: 44440 chars

🧪 Testing daily://week
✅ Concatenated 7 days
   Total length: 98295 chars

🧪 Testing daily://list
✅ Found 30 daily notes (last 30 days)
   Most recent: 2025-10-24, 2025-10-23, 2025-10-22...

✅ All tests passed!
```

---

## Key Learnings from Research

### MCP Protocol Insights

1. **ResourceTemplate API**: Uses URI patterns like `notes://{path}` with optional `list` callback
2. **Client Capabilities**: Claude Code has robust resource support, Desktop has limitations
3. **URI Pattern Matching**: RFC 6570 URI templates, clients construct URIs on-demand
4. **Completion Support**: Templates can provide completion callbacks for smart suggestions

### Best Practices Discovered

1. **Static for reliability**: Use static resources for common workflows (morning brain boot)
2. **Templates for flexibility**: Use templates for dynamic access (historical lookup)
3. **Scheme separation**: Different schemes prevent URI conflicts
4. **Security**: Validate paths, prevent directory traversal (`..` checking)
5. **Error handling**: Graceful degradation for missing files

---

## Future Work

### Phase 2: `notes://` Template Resource

**Purpose**: General-purpose vault access beyond daily notes

```typescript
resourceTemplates: [
  {
    uriTemplate: "notes://{path}",
    name: "Note by Path",
    description: "Access any note in ~/.evans-notes/ by relative path",
    mimeType: "text/markdown",
  },
]

// Examples:
notes://daily/2025-10-24.md
notes://daily/2025-10-23-vat-journey.md
notes://bridges/restoration.md
notes://inbox/scratch.md
```

**Requirements**:
- Add `ListResourceTemplatesRequestSchema` handler
- Add `notes://` handler in `ReadResourceRequestSchema`
- Path traversal security (`..` validation)
- Verify path is within `~/.evans-notes`

### Phase 3: Additional Static Views

```typescript
bridges://recent    - Last 3 bridge documents
projects://list     - JSON list of project directories
tldr://recent       - TLDR summaries for last 3 days
```

---

## Files Modified

- `src/mcp-server.ts` - Added 3 resources, expanded read handler (~120 lines)
- `CLAUDE.md` - Documented MCP resources architecture
- `test-mcp-resources.ts` - Manual test script for validation

---

## References

### MCP Specification & SDK

- MCP TypeScript SDK: https://github.com/modelcontextprotocol/typescript-sdk
- Resource Templates: Uses RFC 6570 URI patterns
- Speakeasy MCP Guide: https://www.speakeasy.com/mcp/building-servers/protocol-reference/resources

### Claude Client Capabilities

- Claude Code: Full MCP resource support, @ mention discovery
- Claude Desktop: Limited template support, static resources work reliably
- Known issues: GitHub #263 (dynamic resources), #1016 (resource discovery)

---

## Ultrathink Notes

**Research approach**: Mixed-intent strategy worked well
1. Phase 1 (discovery): Feature matrix via WebSearch + docs
2. Phase 2 (architecture): Pattern analysis from SDK examples
3. Phase 3 (decision): Concrete recommendation with code sketch

**URI design evolution**:
- Started with "should we use templates?"
- Discovered conflict risk with overlapping patterns
- Explored 3 options (keywords, namespace, schemes)
- User suggested semantic split: `daily://` (curated) vs `notes://` (dynamic)
- Result: Cleaner than any AI-suggested option

**Testing strategy**:
- Created standalone test script (no MCP server needed)
- Verified logic before integration
- All 4 resources tested successfully
- Real data (44KB recent, 98KB week) validates design

**Documentation pattern**:
- Updated CLAUDE.md architecture section
- Added to Recent Implementation timeline
- Created standalone research artifact (this file)
- Future maintainers have full context
