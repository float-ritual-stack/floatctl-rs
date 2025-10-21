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
    description: 'Morning brain boot: Semantic search + recent context + GitHub PR/issue status + daily notes synthesis. Use this for "good morning" check-ins or when the user wants to restore context about where they left off on a project.',
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
    description: "Semantic search across conversation history using pgvector embeddings. Returns messages that are semantically similar to the query.",
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
    description: `Query live active context stream with annotation parsing. Supports cross-client context surfacing (Desktop ↔ Claude Code). Parses ctx::, project::, persona::, connectTo:: and other annotations from messages.

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
