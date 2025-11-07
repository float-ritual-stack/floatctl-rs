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
    // List all project directories
    const entries = await readdir(claudeProjectsDir, { withFileTypes: true });
    const projectDirs = entries
      .filter(e => e.isDirectory())
      .filter(e => !projectFilter || e.name === projectFilter)
      .slice(0, maxProjects);

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
          const lines = content.split("\n");

          const head = lines.slice(0, headLines).join("\n");
          const tail = lines.length > headLines 
            ? lines.slice(-tailLines).join("\n")
            : "";

          snippets.push({
            project: dir.name,
            file,
            mtime,
            headLines: head,
            tailLines: tail,
          });
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

**Recent activity (head)**:
\`\`\`
${snippet.headLines}
\`\`\`

${snippet.tailLines ? `**Recent conclusions (tail)**:
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
  const defaultOptions: ClaudeProjectsContextOptions = {
    projectFilter: options.projectFilter || "-Users-evan--evna",
    maxProjects: 1,  // Start with just evna
    maxFiles: 3,
    headLines: 20,
    tailLines: 10,
    maxAge: 72,  // 3 days
    ...options,
  };

  const snippets = await getClaudeProjectsContext(defaultOptions);
  return formatSnippetsForPrompt(snippets);
}

/**
 * Get context from ALL projects (extended version)
 */
export async function getAllProjectsContextInjection(
  options: ClaudeProjectsContextOptions = {}
): Promise<string> {
  const defaultOptions: ClaudeProjectsContextOptions = {
    maxProjects: 5,  // Top 5 most recently active
    maxFiles: 2,     // 2 files per project
    headLines: 15,
    tailLines: 8,
    maxAge: 48,      // 2 days for broader scan
    ...options,
  };

  const snippets = await getClaudeProjectsContext(defaultOptions);
  return formatSnippetsForPrompt(snippets);
}
