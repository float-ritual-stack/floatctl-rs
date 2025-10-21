/**
 * Shared configuration for all EVNA interfaces (CLI, TUI, MCP)
 * Single source of truth for Agent SDK query options
 */

import { readFileSync } from "fs";
import { join } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";

// Get the directory of this module file
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load EVNA system prompt from project root (two levels up from src/core/)
export const evnaSystemPrompt = (() => {
  const promptPath = join(__dirname, "..", "..", "evna-system-prompt.md");
  try {
    return readFileSync(promptPath, "utf-8");
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    const errorCode = error instanceof Error && 'code' in error ? (error as any).code : undefined;

    throw new Error(
      `Failed to load EVNA system prompt from: ${promptPath}\n\n` +
      `Error: ${errorMessage}\n` +
      (errorCode === 'ENOENT'
        ? `File not found. Expected at project root: evna-system-prompt.md\n`
        : errorCode === 'EACCES'
        ? `Permission denied. Please check file permissions.\n`
        : '') +
      `\nModule directory: ${__dirname}\n` +
      `Resolved path: ${promptPath}`
    );
  }
})();

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
