/**
 * Shared tool registry using Zod schemas
 * TRUE single source of truth - Agent SDK uses directly, MCP converts to JSON
 */

import { z } from "zod";
import { zodToJsonSchema } from "zod-to-json-schema";
import workspaceContextData from "../config/workspace-context.json";

// Type definitions for workspace context config
interface UserConfig {
  name: string;
  github_username: string;
  timezone: string;
  work_hours: {
    start: string;
    end: string;
    timezone: string;
  };
}

interface ProjectConfig {
  canonical: string;
  aliases: string[];
  description: string;
  repo: string;
  type: string;
}

interface MeetingConfig {
  canonical: string;
  aliases: string[];
  description: string;
  project?: string;
  typical_time?: string;
  attendees?: string[];
}

interface PathsConfig {
  daily_notes: string;
  inbox: string;
  operations: string;
}

interface WorkspaceContext {
  user: UserConfig;
  projects: Record<string, ProjectConfig>;
  paths: PathsConfig;
  meetings: Record<string, MeetingConfig>;
  _meta: { note: string; philosophy: string; version: string; last_updated: string };
}

const workspace = workspaceContextData as WorkspaceContext;

// Tool schema definitions
export const toolSchemas = {
  brain_boot: {
    name: "brain_boot" as const,
    description: `Restore project context using semantic search, recent activity, and GitHub status.

**Purpose**: Comprehensive context restoration combining semantic search, active context stream, and optional GitHub PR/issue status.

**When to use**:
- Morning check-ins - "where did I leave off?"
- Switching projects - get quick context
- After time away - "what was I working on last Tuesday?"

**When NOT to use**:
- Deep historical search (use semantic_search)
- Capturing new information (use active_context capture)

**Example**: brain_boot(query: "pharmacy GP node work", project: "pharmacy", lookbackDays: 3)

**Returns**: Markdown synthesis with semantic results, recent context, GitHub summaries (if provided), temporally organized`,
    schema: z.object({
      query: z
        .string()
        .describe(
          'Natural language description of what to retrieve context about (e.g., "tuesday morning pharmacy project where did I leave off")',
        ),
      project: z
        .string()
        .optional()
        .describe('Filter by project name (e.g., "rangle/pharmacy")'),
      lookbackDays: z
        .number()
        .optional()
        .describe("How many days to look back (default: 7)"),
      maxResults: z
        .number()
        .optional()
        .describe("Maximum results to return (default: 10)"),
      githubUsername: z
        .string()
        .optional()
        .describe("GitHub username to fetch PR and issue status (e.g., 'e-schultz')"),
    }),
  },

  semantic_search: {
    name: "semantic_search" as const,
    description: `Deep semantic search across conversation history using pgvector embeddings.

**Purpose**: Find semantically similar messages even when exact keywords don't match or concepts are expressed differently. Searches entire archive.

**When to use**:
- Archaeological code exploration - "where did we discuss error handling patterns?"
- Finding related discussions across time
- Cross-project pattern discovery

**When NOT to use**:
- Recent activity (use brain_boot or active_context)
- Exact string matching (use grep/file search)

**How to query effectively**:

❌ **BAD (keyword soup)**: Stacking terms like "silent scribe whisper draft daemon" or "float block scratch pad quiet mode"

✅ **GOOD (semantic concepts)**:
- "ambient observer that chronicles without demanding attention"
- "buffer stream of consciousness until ready for AI processing"
- "DND mode where I can think out loud without AI interruption"
- "when I'm in DND mode but still burping in chat, give rich thoughts to tools, short confirmation in chat, scratch pad without wall of text interruption"

**Think**: Describe the CONCEPT, not just stack terms. The embedding model understands meaning and context, not keyword presence.

**Example**: semantic_search(query: "Issue 168 GP node rendering", project: "pharmacy", threshold: 0.6)

**Returns**: Markdown list with conversation title, message excerpt, similarity score, metadata, sorted by relevance`,
    schema: z.object({
      query: z
        .string()
        .describe("Search query (can be natural language, a question, or keywords)"),
      limit: z
        .number()
        .optional()
        .describe("Maximum number of results (default: 10)"),
      project: z
        .string()
        .optional()
        .describe("Filter by project name"),
      since: z
        .string()
        .optional()
        .describe('Filter by timestamp (ISO 8601 format, e.g., "2025-10-01T00:00:00Z")'),
      threshold: z
        .number()
        .optional()
        .describe("Similarity threshold 0-1 (default: 0.5, lower = more results)"),
    }),
  },

  active_context: {
    name: "active_context" as const,
    description: `Capture and query recent activity with annotation parsing and cross-client surfacing.

**Purpose**: Real-time context management - capture decisions/insights/state changes, surface recent context between Desktop ↔ Claude Code sessions.

**Dual modes**:
1. **Capture mode** (with \`capture\` parameter): Store annotated messages, parse ctx::, project::, meeting::, issue::, mode:: annotations
2. **Query mode** (with \`query\` parameter): Retrieve recent context with fuzzy project matching and cross-client surfacing

**When to use**:
- Capture: When you see ctx:: or project:: annotations (proactive), after meetings/decisions/insights
- Query: Restore recent work context, check what happened in other client

**When NOT to use**:
- Historical/archived data (use semantic_search)
- File content (use grep/read tools)

**Example**: active_context(query: "GP node rendering", project: "pharmacy", limit: 5)

**Returns**: Markdown stream with timestamp, client badge (💻/💬), role (👤/🤖), project/personas/mode annotations, content preview, extracted highlights`,
    schema: z.object({
      query: z
        .string()
        .optional()
        .describe("Optional search query for filtering context"),
      capture: z
        .string()
        .optional()
        .describe("Capture this message to active context stream (with annotation parsing)"),
      limit: z
        .number()
        .optional()
        .describe("Maximum number of results (default: 10)"),
      project: z
        .string()
        .optional()
        .describe("Filter by project name (extracted from project:: annotations)"),
      client_type: z
        .enum(["desktop", "claude_code"])
        .optional()
        .describe("Filter by client type"),
      include_cross_client: z
        .boolean()
        .optional()
        .describe("Include context from other client (default: true)"),
    }),
  },

  r2_sync: {
    name: "r2_sync" as const,
    description: `Manage R2 sync daemons (daily notes + float.dispatch autosync).

**Purpose**: Monitor and control automatic syncing of daily notes and dispatch content to Cloudflare R2 storage.

**Operations**:
- **status**: Check daemon health, PIDs, last sync times
- **trigger**: Manually force sync (use wait=true to block until complete)
- **start**: Launch stopped daemon(s)
- **stop**: Gracefully stop daemon(s)
- **logs**: View recent sync activity

**When to use**:
- **status**: Check if daemons are running, view last sync times, troubleshoot issues
- **trigger**: Force immediate backup after important changes, test sync, emergency backup
- **start**: After daemon stopped, system startup, recovering from crashes
- **stop**: Before maintenance, temporarily disable sync, before config changes
- **logs**: Debug issues, check recent activity, investigate errors

**When NOT to use**:
- trigger during normal operation (let automatic sync handle it)
- trigger repeatedly in quick succession (respect debounce intervals)

**Examples**:
- r2_sync(operation: "status") → Check all daemons
- r2_sync(operation: "status", daemon_type: "daily") → Check specific daemon
- r2_sync(operation: "trigger", daemon_type: "daily", wait: true) → Manual sync
- r2_sync(operation: "logs", daemon_type: "daily", lines: 20) → View logs

**Returns**: Markdown-formatted results specific to operation`,
    schema: z.object({
      operation: z
        .enum(["status", "trigger", "start", "stop", "logs"])
        .describe("Which sync operation to perform"),
      daemon_type: z
        .enum(["daily", "dispatch", "all"])
        .optional()
        .describe("Which daemon(s) to operate on (default: all for status/trigger/start/stop; required for logs)"),
      wait: z
        .boolean()
        .optional()
        .describe("Wait for sync to complete (trigger operation only, default: false)"),
      lines: z
        .number()
        .optional()
        .describe("Number of log lines to show (logs operation only, default: 20)"),
    }),
  },
};

/**
 * Convert tool schemas to MCP JSON format
 */
export function toMcpTools() {
  return Object.values(toolSchemas).map((tool) => ({
    name: tool.name,
    description: tool.description,
    inputSchema: zodToJsonSchema(tool.schema, { $refStrategy: "none" }),
  }));
}
