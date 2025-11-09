/**
 * Claude Projects Context Injection
 * 
 * Reads recent conversation snippets from ~/.claude/projects to give EVNA
 * "peripheral vision" into recent Desktop/Code work
 */

import { readdir, readFile, stat } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

export interface ClaudeProjectSnippet {
  project: string;
  file: string;
  mtime: Date;
  headLines: string;  // First N lines
  tailLines: string;  // Last N lines
}

export interface ClaudeProjectsContextOptions {
  projectFilter?: string;  // Specific project folder name (e.g., "-Users-evan--evna")
  maxProjects?: number;    // Max number of projects to include (default: 3)
  maxFiles?: number;       // Max files per project (default: 3)
  headLines?: number;      // Lines from start of file (default: 20)
  tailLines?: number;      // Lines from end of file (default: 10)
  maxAge?: number;         // Max file age in hours (default: 72 - 3 days)
}

/**
 * Get recent conversation snippets from Claude projects
 */
export async function getClaudeProjectsContext(
  options: ClaudeProjectsContextOptions = {}
): Promise<ClaudeProjectSnippet[]> {
  const {
    projectFilter,
    maxProjects = 3,
    maxFiles = 3,
    headLines = 20,
    tailLines = 10,
    maxAge = 72, // 3 days
  } = options;

  const claudeProjectsDir = join(homedir(), ".claude", "projects");
  const snippets: ClaudeProjectSnippet[] = [];
  const cutoffTime = Date.now() - (maxAge * 60 * 60 * 1000);

  try {
    // List all project directories and sort by mtime
    const entries = await readdir(claudeProjectsDir, { withFileTypes: true });
    const dirsWithMtime = await Promise.all(
      entries
        .filter(e => e.isDirectory())
        .filter(e => !projectFilter || e.name === projectFilter)
        .map(async (e) => {
          const path = join(claudeProjectsDir, e.name);
          const stats = await stat(path);
          return { dir: e, mtime: stats.mtime };
        })
    );
    
    const projectDirs = dirsWithMtime
      .sort((a, b) => b.mtime.getTime() - a.mtime.getTime())
      .slice(0, maxProjects)
      .map(d => d.dir);

    for (const dir of projectDirs) {
      const projectPath = join(claudeProjectsDir, dir.name);
      
      // Find recent .jsonl files
      const files = await readdir(projectPath);
      const jsonlFiles = files.filter(f => f.endsWith(".jsonl"));

      // Get file stats and sort by mtime
      const fileStats = await Promise.all(
        jsonlFiles.map(async (file) => {
          const filePath = join(projectPath, file);
          const stats = await stat(filePath);
          return { file, path: filePath, mtime: stats.mtime };
        })
      );

      const recentFiles = fileStats
        .filter(f => f.mtime.getTime() > cutoffTime)
        .sort((a, b) => b.mtime.getTime() - a.mtime.getTime())
        .slice(0, maxFiles);

      // Extract head/tail from each file
      for (const { file, path, mtime } of recentFiles) {
        try {
          const content = await readFile(path, "utf-8");
          
          // Parse JSONL and extract user/assistant messages only (skip tool chains)
          // Handle multiple formats:
          // Format 1: {type: "queue-operation", operation: "enqueue", content: "..."}
          // Format 2: {type: "user"/"assistant", message: {...}}
          const messages = content
            .split("\n")
            .filter(line => line.trim())
            .map(line => {
              try {
                const msg = JSON.parse(line);
                
                // Format 1: queue-operation/agent-response with content field
                if (msg.type === 'queue-operation' && msg.operation === 'enqueue' && msg.content) {
                  return `👤 ${msg.content}`;
                } else if (msg.type === 'agent-response' && msg.content) {
                  return `🤖 ${msg.content}`;
                }
                
                // Format 2: user/assistant with message field (Claude Code format)
                if (msg.type === 'user' && msg.message) {
                  let text = '';
                  if (typeof msg.message === 'string') {
                    text = msg.message;
                  } else if (msg.message.content) {
                    // content can be string or array of content blocks
                    if (typeof msg.message.content === 'string') {
                      text = msg.message.content;
                    } else if (Array.isArray(msg.message.content)) {
                      // Extract text blocks only (skip tool_use, etc)
                      text = msg.message.content
                        .filter((b: any) => b.type === 'text')
                        .map((b: any) => b.text)
                        .join(' ');
                    }
                  }
                  if (text.trim()) return `👤 ${text.substring(0, 500)}`;
                } else if (msg.type === 'assistant' && msg.message) {
                  let text = '';
                  if (typeof msg.message === 'string') {
                    text = msg.message;
                  } else if (msg.message.content) {
                    if (typeof msg.message.content === 'string') {
                      text = msg.message.content;
                    } else if (Array.isArray(msg.message.content)) {
                      text = msg.message.content
                        .filter((b: any) => b.type === 'text')
                        .map((b: any) => b.text)
                        .join(' ');
                    }
                  }
                  if (text.trim()) return `🤖 ${text.substring(0, 500)}`;
                }
                
                return null;
              } catch {
                return null;
              }
            })
            .filter((msg): msg is string => msg !== null);

          const head = messages.slice(0, headLines).join("\n");
          const tail = messages.length > headLines 
            ? messages.slice(-tailLines).join("\n")
            : "";

          // Only add snippet if we have actual content
          if (head || tail) {
            snippets.push({
              project: dir.name,
              file,
              mtime,
              headLines: head,
              tailLines: tail,
            });
          }
        } catch (error) {
          // Skip files that can't be read
          console.error(`[claude-projects-context] Error reading ${path}:`, error);
        }
      }
    }

    return snippets;
  } catch (error) {
    console.error("[claude-projects-context] Error reading projects:", error);
    return [];
  }
}

/**
 * Format snippets as markdown for injection into system prompt
 */
export function formatSnippetsForPrompt(snippets: ClaudeProjectSnippet[]): string {
  if (snippets.length === 0) {
    return "";
  }

  const sections = snippets.map(snippet => {
    const timestamp = snippet.mtime.toLocaleString("en-US", {
      timeZone: "America/Toronto",
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });

    return `
### ${snippet.project} / ${snippet.file}
**Modified**: ${timestamp}

${snippet.headLines ? `**Request (head)**:
\`\`\`
${snippet.headLines}
\`\`\`

` : ""}${snippet.tailLines ? `**Response (tail)**:
\`\`\`
${snippet.tailLines}
\`\`\`` : ""}
`;
  });

  return `
---

## Recent Claude Desktop/Code Activity

The following snippets show recent work from Claude projects. Use this to check:
- "Have I answered this question recently?"
- "What context from other sessions is relevant?"
- "Are there patterns across different work streams?"

${sections.join("\n---\n")}

---
`;
}

/**
 * Get formatted context for ask_evna injection
 * Focused on evna project by default, can expand to all projects
 */
export async function getAskEvnaContextInjection(
  options: ClaudeProjectsContextOptions = {}
): Promise<string> {
  // Default to evna project for focused context
  // User/assistant messages only (noise filtered)
  const defaultOptions: ClaudeProjectsContextOptions = {
    projectFilter: options.projectFilter || "-Users-evan--evna",
    maxProjects: 1,  // Just evna
    maxFiles: 3,
    headLines: 3,    // First 3 user/assistant messages
    tailLines: 3,    // Last 3 user/assistant messages
    maxAge: 72,      // 3 days
    ...options,
  };

  const snippets = await getClaudeProjectsContext(defaultOptions);
  return formatSnippetsForPrompt(snippets);
}

/**
 * Get context from ALL projects (extended version)
 * Tail-only: Most recent updates across work streams (user/assistant messages filtered)
 */
export async function getAllProjectsContextInjection(
  options: ClaudeProjectsContextOptions = {}
): Promise<string> {
  const defaultOptions: ClaudeProjectsContextOptions = {
    maxProjects: 5,  // Top 5 most recently active
    maxFiles: 3,     // 3 files per project (increased for better coverage)
    headLines: 0,    // No head needed for ambient awareness
    tailLines: 10,   // Last 10 messages (tool-heavy files need more to find text)
    maxAge: 48,      // 2 days for broader scan
    ...options,
  };

  const snippets = await getClaudeProjectsContext(defaultOptions);
  return formatSnippetsForPrompt(snippets);
}
