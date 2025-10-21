/**
 * Shared configuration for all EVNA interfaces (CLI, TUI, MCP)
 * Single source of truth for Agent SDK query options
 */

import { readFileSync } from "fs";
import { join } from "path";

// Load EVNA system prompt from project root
// Using process.cwd() ensures it works regardless of where the module is compiled/loaded
export const evnaSystemPrompt = readFileSync(
  join(process.cwd(), "evna-system-prompt.md"),
  "utf-8"
);

// Default model
export const DEFAULT_MODEL = "claude-sonnet-4-20250514";

/**
 * Create standardized Agent SDK query options
 * Used by CLI, TUI, and MCP interfaces to ensure consistency
 */
export function createQueryOptions(mcpServer: any) {
  return {
    settingSources: ["user"] as ["user"],
    systemPrompt: {
      type: "preset" as const,
      preset: "claude_code" as const,
      append: evnaSystemPrompt,
    },
    mcpServers: {
      "evna-next": mcpServer,
    },
    model: DEFAULT_MODEL,
    permissionMode: "bypassPermissions" as const, // Auto-approve all tools
  };
}
