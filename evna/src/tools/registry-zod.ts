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
    description: `Comprehensive context restoration for session start or project switching. Combines semantic search, active context stream, and optional GitHub PR/issue status. Use for morning check-ins, after time away, or when switching projects.`,
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
    description: `Deep semantic search across conversation history using pgvector embeddings. Finds semantically similar messages even when keywords don't match. Searches entire archive. Use for archaeological exploration, finding related discussions, cross-project patterns. Describe concepts not keyword soup.`,
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
    description: `Real-time context management - capture decisions/insights/state changes, query recent activity. Parses ctx::, project::, mode:: annotations. Uses Ollama for synthesis. Capture after work, query for recent context, or both together.`,
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
    description: `Manage R2 sync daemons for daily notes + float.dispatch. Operations: status, trigger, start, stop, logs. Use for troubleshooting sync issues or forcing immediate backup.`,
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
    description: `LLM-driven orchestrator for natural language questions about work context. Coordinates multiple sources (semantic search, activity, daily notes, GitHub). Supports multi-turn conversations via session_id. Use for complex investigations, open-ended queries, or unclear intent.`,
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

  peek_session: {
    name: "peek_session" as const,
    description: `Read-only view of evna session progress without resuming agent loop. Check if timed-out session finished, view partial results, or inspect completed sessions.`,
    schema: z.object({
      session_id: z
        .string()
        .describe("Session ID to peek at (from ask_evna timeout or response)"),
      message_count: z
        .number()
        .optional()
        .describe("Number of recent messages to show (default: 5)"),
      include_tools: z
        .boolean()
        .optional()
        .describe("Include tool calls in output (default: false - messages only)"),
    }),
  },

  update_system_prompt: {
    name: "update_system_prompt" as const,
    description: `Update EVNA's system prompt for self-modification. Saves to ~/.evna/system-prompt.md with automatic backup. Changes take effect on restart. Only use when explicitly requested.`,
    schema: z.object({
      content: z.string().describe("New system prompt content (full replacement)"),
      backup: z.boolean().optional().describe("Create timestamped backup (default: true)"),
    }),
  },

  read_system_prompt: {
    name: "read_system_prompt" as const,
    description: `Read EVNA's current system prompt from ~/.evna/system-prompt.md.`,
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
