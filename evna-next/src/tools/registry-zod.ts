/**
 * Shared tool registry using Zod schemas
 * TRUE single source of truth - Agent SDK uses directly, MCP converts to JSON
 */

import { z } from "zod";
import { zodToJsonSchema } from "zod-to-json-schema";
import normalizationData from "../config/normalization.json";

// Type definitions for normalization config
interface ProjectConfig {
  canonical: string;
  aliases: string[];
  description: string;
}

interface NormalizationConfig {
  projects: Record<string, ProjectConfig>;
  meetings: Record<string, { canonical: string; aliases: string[]; description: string }>;
  _meta: { note: string; philosophy: string };
}

const normalization = normalizationData as NormalizationConfig;

/**
 * Generate normalization examples for tool descriptions
 * Philosophy: LLMs as fuzzy compilers - provide examples, embrace deviation
 */
function buildNormalizationExamples(): string {
  const projectExamples = Object.entries(normalization.projects)
    .map(([key, { canonical, aliases }]) =>
      `  - "${canonical}": ${aliases.map((a: string) => `"${a}"`).join(', ')}`
    )
    .join('\n');

  return `
IMPORTANT: These are EXAMPLE patterns of how the user typically names things.
The user WILL deviate from these - fuzzy match generously, don't enforce rigidity.

Common project variations (normalize when capturing, fuzzy match when querying):
${projectExamples}

Philosophy: "LLMs as fuzzy compilers" - bring structure to the mess, don't fight it.`.trim();
}

// Tool schema definitions
export const toolSchemas = {
  brain_boot: {
    name: "brain_boot" as const,
    description: `Morning brain boot: Restore project context using semantic search, recent activity, and GitHub status.

**Purpose**: Comprehensive context restoration combining:
- Semantic search across conversation history
- Recent active context stream messages
- GitHub PR and issue status (if username provided)
- Daily notes and meeting summaries

**When to use**:
- "Good morning" check-ins - restore where you left off
- Switching between projects - get quick context
- After time away - "what was I working on last Tuesday?"
- Project status updates - combine recent activity with PR status

**When NOT to use**:
- For deep historical search (use semantic_search instead)
- For capturing new information (use active_context capture instead)
- For listing all conversations (too broad, be specific)

**Usage Examples**:
\`\`\`
brain_boot(query: "pharmacy project GP node work", project: "pharmacy")
brain_boot(query: "tuesday morning floatctl where did I leave off", lookbackDays: 3)
brain_boot(query: "issue 168 status", githubUsername: "e-schultz")
\`\`\`

**Returns**: Markdown-formatted synthesis with:
- Semantic search results (most relevant messages)
- Recent active context entries
- GitHub PR/issue summaries (if applicable)
- Temporal organization with timestamps

**Error Handling**:
- If empty: Try broader query, increase lookbackDays, or remove project filter
- If too much data: Add project filter or reduce maxResults
- GitHub errors: Verify username is correct, check API rate limits`,
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

**Purpose**: Find semantically similar messages from past conversations, even when:
- Exact keywords don't match (finds "API integration" for query "web service connection")
- Concepts are expressed differently (finds "authentication bug" for "login failure")
- Long-term historical search needed (searches entire archive)

**When to use**:
- Archaeological code exploration - "where did we discuss error handling patterns?"
- Finding related discussions - "conversations about Redux dispatches"
- Technical archaeology - "when did we implement annotation parsing?"
- Cross-project pattern discovery - "how have we solved pagination before?"

**When NOT to use**:
- Recent activity (use brain_boot or active_context instead)
- Exact string matching (use grep/file search)
- Broad exploration without specific query (be targeted)

**Usage Examples**:
\`\`\`
semantic_search(query: "annotation parsing JSONB schema", limit: 15)
semantic_search(query: "Issue 168 GP node rendering", project: "pharmacy", threshold: 0.6)
semantic_search(query: "fuzzy compiler philosophy redux", since: "2025-10-01T00:00:00Z")
\`\`\`

**Returns**: Markdown-formatted list with:
- Conversation title and date
- Matched message excerpt (context around match)
- Similarity score (higher = more relevant)
- Message metadata (project, meeting, markers)
- Sorted by relevance (most similar first)

**Error Handling**:
- If empty: Lower threshold (try 0.3-0.4), broaden query, remove filters
- Too many results: Increase threshold (try 0.6-0.7), add project filter, reduce limit
- No results but data exists: Check spelling, try synonyms, use broader terms`,
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
    description: `Live context stream: Capture and query recent activity with rich annotation parsing and cross-client surfacing.

**Purpose**: Real-time context management for:
- Capturing important decisions, insights, and state changes
- Surfacing recent context between Desktop ↔ Claude Code sessions
- Tracking project/meeting/mode annotations
- Building narrative thread across work sessions

**Dual modes**:
1. **Capture mode** (with \`capture\` parameter): Store annotated messages
   - Parses ctx::, project::, meeting::, issue::, mode::, etc.
   - Normalizes project names to canonical form
   - Extracts highlights, personas, patterns from annotations

2. **Query mode** (with \`query\` parameter): Retrieve recent context
   - Fuzzy matches project names (e.g., "evna" finds "floatctl/evna" entries)
   - Cross-client surfacing (Desktop sees Claude Code context, vice versa)
   - Temporal ordering (most recent first)

**When to use**:
- **Capture (PROACTIVE)**: When you see messages with ctx::, project::, meeting:: annotations - AUTOMATICALLY capture them
- **Capture (manual)**: After meetings, decisions, insights, state changes, bug discoveries
- **Query**: Restore recent work context, see what happened in other client, check project activity
- Both modes work together: Capture decisions in Desktop, query them in Claude Code

**IMPORTANT - Proactive Capture Rule**:
If a user message contains ctx:: or project:: annotations, you should IMMEDIATELY use active_context
with the capture parameter to store it. Don't wait for the user to ask - they've already formatted
it for capture by adding annotations.

**When NOT to use**:
- For historical/archived data (use semantic_search for deep history)
- For file content (use grep/read tools)
- Without annotations (format: \`ctx::YYYY-MM-DD @ HH:MM [project::name]\`)

**Usage Examples**:
\`\`\`
// Capture meeting decisions
active_context(capture: "ctx::2025-10-21 @ 02:00 PM [meeting::pharmacy/scott-sync] [project::pharmacy] [issue::168]

Scott sync decisions:
- Rename 'GP Surgery Details' → 'Notify GP'
- Fix switch node rendering (hide children until parent selected)")

// Query recent pharmacy project activity
active_context(query: "GP node rendering", project: "pharmacy", limit: 5)

// Get cross-client context (Desktop sees Claude Code work)
active_context(project: "floatctl", include_cross_client: true)

// Capture with highlights
active_context(capture: "ctx::2025-10-21 @ 02:30 PM [project::floatctl/evna]
highlight::Normalization pattern working! Project name fuzzy matching reduces query friction.")
\`\`\`

**Returns**: Markdown-formatted stream with:
- Message timestamp and client badge (💻 = Claude Code, 💬 = Desktop)
- Role indicator (👤 = user, 🤖 = assistant)
- Project, personas, mode annotations
- Content preview (first 200 chars)
- Highlights and patterns extracted

**Error Handling**:
- If empty: Remove project filter, check if data was captured, verify annotation format
- Cross-client not working: Check include_cross_client=true (default)
- Project not matching: Try aliases (e.g., "evna", "floatctl", "float/evna" all work)
- Format issues: Ensure annotations use :: separator (project::name, not project:name)

${buildNormalizationExamples()}`,
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
