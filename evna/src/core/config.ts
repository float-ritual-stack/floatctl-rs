/**
 * Shared configuration for all EVNA interfaces (CLI, TUI, MCP)
 * Single source of truth for Agent SDK query options
 */

import { readFileSync } from "fs";
import { join } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";
import { homedir } from "os";

// Get the directory of this module file
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Load EVNA system prompt from ~/.evna/ or fallback to project root
// This allows evna to update her own system prompt
export const evnaSystemPrompt = (() => {
  // Primary location: ~/.evna/system-prompt.md (user-editable, persists across updates)
  const userPromptPath = join(homedir(), ".evna", "system-prompt.md");
  
  // Fallback location: project root (default template)
  const projectPromptPath = join(__dirname, "..", "..", "evna-system-prompt.md");
  
  try {
    // Try user location first
    return readFileSync(userPromptPath, "utf-8");
  } catch (userError) {
    // Fallback to project location
    try {
      return readFileSync(projectPromptPath, "utf-8");
    } catch (projectError) {
      const errorMessage = projectError instanceof Error ? projectError.message : String(projectError);
      const errorCode = projectError instanceof Error && 'code' in projectError ? (projectError as any).code : undefined;

      throw new Error(
        `Failed to load EVNA system prompt from both locations:\n\n` +
        `1. User location: ${userPromptPath}\n` +
        `2. Project location: ${projectPromptPath}\n\n` +
        `Error: ${errorMessage}\n` +
        (errorCode === 'ENOENT'
          ? `File not found. Run: mkdir -p ~/.evna && cp evna-system-prompt.md ~/.evna/system-prompt.md\n`
          : errorCode === 'EACCES'
          ? `Permission denied. Please check file permissions.\n`
          : '') +
        `\nModule directory: ${__dirname}`
      );
    }
  }
})();

// Get the active system prompt path (for tools that need to update it)
export function getSystemPromptPath(): string {
  const userPromptPath = join(homedir(), ".evna", "system-prompt.md");
  const projectPromptPath = join(__dirname, "..", "..", "evna-system-prompt.md");
  
  // Return whichever exists (prefer user location)
  try {
    readFileSync(userPromptPath, "utf-8");
    return userPromptPath;
  } catch {
    return projectPromptPath;
  }
}

// Combined system prompt (no weekly bridge injection - removed hard-coded date)
export function getFullSystemPrompt(): string {
  return evnaSystemPrompt;
}

// Default model
export const DEFAULT_MODEL = "claude-sonnet-4-20250514";

// Max turns for agent loops - prevents runaway token burns
// 25 allows complex orchestration while stopping graveyard excavations
export const DEFAULT_MAX_TURNS = 25;

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
      append: getFullSystemPrompt(), // Dynamic: reloads weekly bridge on every call
    },
    mcpServers: {
      "evna-next": mcpServer,
    },
    model: DEFAULT_MODEL,
    maxTurns: DEFAULT_MAX_TURNS, // Prevent token burns on runaway loops
    permissionMode: "bypassPermissions" as const, // Auto-approve all tools
  };
}
