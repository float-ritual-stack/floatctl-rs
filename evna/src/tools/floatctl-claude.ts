/**
 * floatctl-claude integration
 * Shells out to floatctl claude commands for Claude Code session log querying
 *
 * Security: Uses execFile (not exec) to prevent shell injection
 */

import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

export interface ClaudeSession {
  session_id: string;
  project: string;
  branch?: string;
  started: string;
  ended?: string;
  turn_count: number;
  tool_calls: number;
}

export interface ClaudeMessage {
  role: string;
  content: string;
  truncated: boolean;
}

export interface ClaudeSessionContext {
  project: string;
  branch?: string;
  first_messages: ClaudeMessage[];
  last_messages: ClaudeMessage[];
  stats: {
    turn_count: number;
    tool_calls: number;
  };
}

export class FloatctlClaudeTool {
  /**
   * List recent Claude Code sessions
   */
  async listRecentSessions(options: {
    n?: number;
    project?: string;
  }): Promise<string> {
    const { n = 10, project } = options;

    const args = [
      "claude",
      "list-sessions",
      "--format",
      "json",
      "--limit",
      String(n),
    ];

    if (project) {
      args.push("--project", project);
    }

    try {
      const { stdout } = await execFileAsync("floatctl", args);
      const sessions = JSON.parse(stdout) as ClaudeSession[];

      // Format for display
      if (sessions.length === 0) {
        return "No Claude Code sessions found.";
      }

      const formatted = sessions
        .map((s, idx) => {
          const started = new Date(s.started).toLocaleString();
          const branchInfo = s.branch ? ` (${s.branch})` : "";
          const stats = `${s.turn_count} turns, ${s.tool_calls} tool calls`;
          return `${idx + 1}. **${started}**\n   Project: ${s.project}${branchInfo}\n   Stats: ${stats}\n   ID: ${s.session_id}`;
        })
        .join("\n\n");

      return `# Recent Claude Code Sessions (${sessions.length})\n\n${formatted}`;
    } catch (error) {
      return this.handleError(error, "listing sessions");
    }
  }

  /**
   * Read recent Claude Code context (first/last messages)
   */
  async readRecentContext(options: {
    sessions?: number;
    first?: number;
    last?: number;
    truncate?: number;
    project?: string;
  }): Promise<string> {
    const {
      sessions = 3,
      first = 3,
      last = 3,
      truncate = 400,
      project,
    } = options;

    const args = [
      "claude",
      "recent-context",
      "--format",
      "json",
      "--sessions",
      String(sessions),
      "--first",
      String(first),
      "--last",
      String(last),
      "--truncate",
      String(truncate),
    ];

    if (project) {
      args.push("--project", project);
    }

    try {
      const { stdout } = await execFileAsync("floatctl", args);
      const data = JSON.parse(stdout) as {
        sessions: ClaudeSessionContext[];
      };

      if (data.sessions.length === 0) {
        return "No Claude Code sessions found.";
      }

      // Format for display
      const formatted = data.sessions
        .map((s) => {
          const branchInfo = s.branch ? ` (${s.branch})` : "";
          const firstMsgs = s.first_messages
            .map(
              (m) =>
                `  [${m.role}]: ${m.content}${m.truncated ? "..." : ""}`
            )
            .join("\n");

          const lastMsgs = s.last_messages
            .map(
              (m) =>
                `  [${m.role}]: ${m.content}${m.truncated ? "..." : ""}`
            )
            .join("\n");

          return (
            `## Session: ${s.project}${branchInfo}\n\n` +
            `**First messages:**\n${firstMsgs}\n\n` +
            `**Last messages:**\n${lastMsgs}\n\n` +
            `**Stats:** ${s.stats.turn_count} turns, ${s.stats.tool_calls} tool calls`
          );
        })
        .join("\n\n---\n\n");

      return `# Recent Claude Code Context (${data.sessions.length} sessions)\n\n${formatted}`;
    } catch (error) {
      return this.handleError(error, "reading context");
    }
  }

  /**
   * Handle errors with proper error code detection
   */
  private handleError(error: unknown, operation: string): string {
    if (!(error instanceof Error)) {
      return `Error ${operation}: ${String(error)}`;
    }

    // Use error.code for more reliable error detection
    const errorWithCode = error as NodeJS.ErrnoException;

    // Command not found
    if (errorWithCode.code === "ENOENT") {
      return "Error: floatctl command not found. Please ensure floatctl is installed and in your PATH.\n\nInstall with: cargo install --path floatctl-cli";
    }

    // Parse stderr for Rust error messages (more specific than parsing error.message)
    if (errorWithCode.message.includes("Failed to open history file")) {
      return "Error: Claude Code history file not found at ~/.claude/history.jsonl\n\nEnsure Claude Code has been run at least once.";
    }

    if (errorWithCode.message.includes("Failed to parse")) {
      return `Error: Failed to parse floatctl output. The history file may be corrupted.\n\n${errorWithCode.message}`;
    }

    // Generic error with full context
    return `Error ${operation}: ${errorWithCode.message}`;
  }
}
