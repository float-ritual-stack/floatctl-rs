/**
 * Internal-only tool schemas
 * These tools are available to ask_evna's agent but not exposed to external MCP clients
 */

import { z } from "zod";

export const internalToolSchemas = {
  github_read_issue: {
    name: "github_read_issue" as const,
    description: `Read a GitHub issue from any repository. No restrictions - can read from any repo you have access to.`,
    schema: z.object({
      repo: z.string().describe('Repository in format "owner/name" (e.g., "float-ritual-stack/float-hub")'),
      number: z.number().describe("Issue number"),
    }),
  },

  github_comment_issue: {
    name: "github_comment_issue" as const,
    description: `Post a comment to a GitHub issue. Write access restricted to float-ritual-stack/* repositories only.`,
    schema: z.object({
      repo: z.string().describe('Repository in format "owner/name" (e.g., "float-ritual-stack/float-hub")'),
      number: z.number().describe("Issue number"),
      body: z.string().describe("Comment body (supports Markdown)"),
    }),
  },

  github_close_issue: {
    name: "github_close_issue" as const,
    description: `Close a GitHub issue. Write access restricted to float-ritual-stack/* repositories only. Optionally include a closing comment.`,
    schema: z.object({
      repo: z.string().describe('Repository in format "owner/name" (e.g., "float-ritual-stack/float-hub")'),
      number: z.number().describe("Issue number"),
      comment: z.string().optional().describe("Optional comment when closing"),
    }),
  },

  github_add_label: {
    name: "github_add_label" as const,
    description: `Add a label to a GitHub issue. Write access restricted to float-ritual-stack/* repositories only.`,
    schema: z.object({
      repo: z.string().describe('Repository in format "owner/name" (e.g., "float-ritual-stack/float-hub")'),
      number: z.number().describe("Issue number"),
      label: z.string().describe("Label name to add"),
    }),
  },

  github_remove_label: {
    name: "github_remove_label" as const,
    description: `Remove a label from a GitHub issue. Write access restricted to float-ritual-stack/* repositories only.`,
    schema: z.object({
      repo: z.string().describe('Repository in format "owner/name" (e.g., "float-ritual-stack/float-hub")'),
      number: z.number().describe("Issue number"),
      label: z.string().describe("Label name to remove"),
    }),
  },

  bridge_health: {
    name: "bridge_health" as const,
    description: `Analyze bridge documents for maintenance needs using Ollama (cost-free).

**Purpose**: Bridge gardening - keep knowledge base healthy with atomic, well-connected notes.

**Analysis types**:
- **duplicates**: Find similar bridges that should be merged (uses Ollama embeddings)
- **large**: Detect bridges >10KB that need splitting into atomic notes
- **stale**: Find bridges not updated in 90+ days (archive candidates)
- **ready_for_imprint**: Bridges mature enough for promotion to imprints/zines (Ollama scoring)
- **all**: Run all analyses (comprehensive health check)

**Requirements**: Ollama running locally with qwen2.5:7b and nomic-embed-text models`,
    schema: z.object({
      report_type: z
        .enum(["duplicates", "large", "stale", "ready_for_imprint", "all"])
        .optional()
        .describe("Type of health check to run (default: all)"),
      max_age_days: z
        .number()
        .optional()
        .describe("Days before bridge considered stale (default: 90)"),
      large_threshold_kb: z
        .number()
        .optional()
        .describe("Size in KB before bridge considered too large (default: 10)"),
    }),
  },
};
