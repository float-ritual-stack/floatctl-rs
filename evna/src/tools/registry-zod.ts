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
    description: `Capture and query recent activity with intelligent synthesis.

**Purpose**: Real-time context management - capture decisions/insights/state changes, synthesize relevant recent context.

**Usage patterns**:
1. **Capture only**: Store annotated messages, parse ctx::, project::, meeting::, issue::, mode:: annotations
2. **Query only**: Synthesize recent context relevant to query (uses Ollama - cost-free)
3. **Capture + Query together** (RECOMMENDED): Log what you just did, then see what came before - enables "store this completion, show me related work" workflows

**When to use**:
- Capture: When you see ctx:: or project:: annotations (proactive), after meetings/decisions/insights
- Query: Restore recent work context, check what happened in other client
- **Capture + Query**: After completing work ("I just finished X, what was I working on before?"), context switches ("logging this, what's next?"), insight connections ("store this discovery, show related patterns")

**When NOT to use**:
- Historical/archived data (use semantic_search)
- File content (use grep/read tools)

**Synthesis behavior**:
- Filters out irrelevant messages
- Avoids repeating user's query back to them (just-captured message won't appear in same query)
- Highlights patterns, decisions, and relevant context only
- Falls back to raw format if Ollama unavailable

**Examples**:
- Capture + Query: active_context(capture: "ctx:: Issue #656 Phase 1 complete", query: "What's next for Issue #656?", project: "pharmacy")
- Query only: active_context(query: "GP node rendering", project: "pharmacy", limit: 5)
- Capture only: active_context(capture: "ctx:: Meeting notes...")

**Returns**: Synthesized context summary (if query provided), or formatted message stream (if capture only)`,
    schema: z.object({
      query: z
        .string()
        .optional()
        .describe("Query for contextual synthesis - recent activity relevant to this question"),
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
      synthesize: z
        .boolean()
        .optional()
        .describe("Synthesize context with Ollama vs raw format (default: true)"),
      include_peripheral: z
        .boolean()
        .optional()
        .describe("Include peripheral context (daily notes, other projects) for ambient awareness (default: true)"),
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

  ask_evna: {
    name: "ask_evna" as const,
    description: `Ask evna natural language questions about work context. LLM-driven orchestrator that interprets intent and coordinates multiple context sources (semantic search, recent activity, daily notes, GitHub status, filesystem).

**Core capabilities**:
- Understands query intent (temporal, project-based, semantic, filesystem, structural)
- Decides which tool(s) to use and chains them for complex queries
- Synthesizes narrative responses (not raw data dumps)
- Filters noise and focuses on relevance
- **Multi-turn conversations**: Remembers full conversation history within sessions
- **Timeout handling**: Returns early if query takes too long (MCP safe)

**When to use ask_evna**:
- Open-ended queries: "summarize all work on X"
- Multi-source composition: "show me everything about Y"
- Complex investigations requiring multiple tool calls
- Follow-up questions building on previous context
- Unclear intent - let evna figure out the approach

**When NOT to use ask_evna**:
- You know exact tool needed (use direct tool for faster response)
- Debugging/testing specific tool behavior
- Simple single-source queries

**Multi-turn conversation workflow**:
1. First question: "Help me debug Issue #123" → returns session_id
2. Follow-up: "What about the related tests?" + session_id → continues with context
3. Branch: "Try different approach" + session_id + fork_session=true → new direction

**Timeout handling (for MCP)**:
- Set timeout_ms (e.g., 25000 for 25 seconds)
- If exceeded, returns session_id with "still processing" message
- Resume with session_id to get results or continue conversation

**Example queries**:
- "What was I working on yesterday afternoon?"
- "Summarize pharmacy Issue #633 discussion"
- "Show me all GP node work across projects"
- "What's blocking the pharmacy release?"
- "Find all notes from 2025-10-31" (uses filesystem tools)
- "What tool usage patterns did I discover this week?" (may create/update bridges)

**Returns**: Synthesized narrative + session_id for continuation`,
    schema: z.object({
      query: z
        .string()
        .describe("Natural language question about your work context"),
      session_id: z
        .string()
        .optional()
        .describe("Session ID to resume previous conversation"),
      fork_session: z
        .boolean()
        .optional()
        .describe("Fork from session_id instead of continuing (default: false)"),
      timeout_ms: z
        .number()
        .optional()
        .describe("Max execution time in milliseconds before returning 'still processing' (default: 60000 - 1 minute for MCP)"),
      include_projects_context: z
        .boolean()
        .optional()
        .describe("Inject recent Claude Desktop/Code conversation snippets for 'peripheral vision' (default: true)"),
      all_projects: z
        .boolean()
        .optional()
        .describe("Include all Claude projects vs just evna project (default: false - evna only)"),
    }),
  },



  update_system_prompt: {
    name: "update_system_prompt" as const,
    description: `Update EVNA's system prompt for self-modification experiments.

**Purpose**: Allow EVNA to update her own identity, behavior guidelines, and knowledge base.

**When to use**:
- User explicitly asks EVNA to update her system prompt
- Experimenting with identity/behavior changes
- Adding new project context or conventions
- Documenting new patterns for future sessions

**When NOT to use**:
- Normal operation (don't spontaneously rewrite yourself)
- Temporary context (use active_context instead)
- Unless explicitly requested by user

**Important**:
- Changes take effect on next session (restart required)
- Creates automatic backup with timestamp
- Saves to ~/.evna/system-prompt.md (persists across code updates)

**Example**: update_system_prompt(content: "...updated prompt...", backup: true)

**Returns**: Confirmation with file path and reload instructions`,
    schema: z.object({
      content: z.string().describe("New system prompt content (full replacement)"),
      backup: z.boolean().optional().describe("Create timestamped backup (default: true)"),
    }),
  },

  read_system_prompt: {
    name: "read_system_prompt" as const,
    description: `Read EVNA's current system prompt.

**Purpose**: View current identity, behavior guidelines, and knowledge base before making changes.

**When to use**:
- Before updating system prompt (to see what's there)
- User asks "what's in your system prompt?"
- Debugging unexpected behavior

**Returns**: Full current system prompt text`,
    schema: z.object({}),
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
