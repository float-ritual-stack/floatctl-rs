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
export const DEFAULT_MODEL = "claude-3-5-haiku-20241022";

// Max turns for agent loops - prevents runaway token burns
// 25 allows complex orchestration while stopping graveyard excavations
export const DEFAULT_MAX_TURNS = 25;

/**
 * Core programmatic agent definitions
 * These are always available, even if ~/.evna/agents/ is empty.
 * Filesystem agents from ~/.evna/agents/ are merged with these.
 *
 * Per SDK docs: subagents cannot spawn their own subagents,
 * so none of these include "Task" in their tools.
 */
export interface AgentDefinition {
  description: string;
  prompt: string;
  tools?: string[];
  model?: 'sonnet' | 'opus' | 'haiku' | 'inherit';
}

export const CORE_AGENTS: Record<string, AgentDefinition> = {
  'quick-search': {
    description: 'Fast file and content search. Use for quick lookups when you need to find files or grep for patterns.',
    prompt: `You are a fast search specialist for the FLOAT ecosystem.
Your job is to quickly find files and content without over-exploring.

Guidelines:
- Use Grep for content patterns, Glob for file patterns
- Return results concisely with file paths and line numbers
- Stop when you've found what's needed - don't exhaustively search
- Prefer BBS bridges (/opt/float/bbs/) for curated knowledge`,
    tools: ['Grep', 'Glob', 'Read'],
    model: 'haiku'
  },

  'bridge-reader': {
    description: 'Read and analyze FLOAT bridge documents. Use when you need to understand bridge content or extract insights from curated knowledge.',
    prompt: `You are a bridge document analyst for the FLOAT ecosystem.
Bridges are curated knowledge documents in /opt/float/bbs/dispatch/bridges/.

When reading bridges:
- Extract key insights and patterns
- Note the bridge status (active, archived, needs-update)
- Identify connections to other bridges
- Summarize actionable information`,
    tools: ['Read', 'Grep', 'Glob'],
    model: 'sonnet'
  },

  'context-capturer': {
    description: 'Capture important context to active_context. Use when significant insights, decisions, or patterns should be preserved for future sessions.',
    prompt: `You are a context preservation specialist.
Your job is to capture important moments, decisions, and patterns using active_context.

Capture format:
ctx::[timestamp] - mode::[mode]
project::[project] | [brief description]

What to capture:
- Significant decisions with rationale
- Pattern discoveries
- Phase transitions
- Failures and what was learned`,
    tools: ['Read'],  // MCP tools (active_context) provided via mcpServers
    model: 'haiku'
  }
};

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
      // Float BBS - agent bulletin board system
      // Evna can write to: evna/, common/, inbox/*, buckets/, priorities/
      // Evna can read: all agent spaces (cowboy, kitty, daddy)
      // Paths: /projects/{evna,common,inbox,buckets,priorities,cowboy,kitty,daddy}
      "float-bbs": {
        type: "stdio" as const,
        command: "/bin/bash",
        args: ["/opt/float/bin/mcp-bbs-evna"],
      },
    },
    model: DEFAULT_MODEL,
    maxTurns: DEFAULT_MAX_TURNS, // Prevent token burns on runaway loops
    permissionMode: "bypassPermissions" as const, // Auto-approve all tools
  };
}
